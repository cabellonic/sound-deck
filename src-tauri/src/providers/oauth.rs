//! OAuth2 de Freesound: autorizacion, intercambio y refresco de tokens (§11).
//!
//! Freesound expone las previews con la API key sola, pero el archivo
//! **original** exige que el usuario autorice la aplicacion con su cuenta. El
//! flujo es el de codigo de autorizacion, sin PKCE porque Freesound no lo
//! soporta y de todas formas el secreto vive en la maquina del usuario.
//!
//! No levantamos un servidor local para el redirect. Freesound permite
//! configurar la credencial para que la pagina de autorizacion **muestre el
//! codigo en pantalla**, y el usuario lo pega en la aplicacion: una cosa menos
//! que pueda fallar por un puerto ocupado o un firewall, y ademas nada escucha
//! en la maquina del usuario.
//!
//! Los tokens quedan en la base junto al resto de la configuracion del
//! proveedor y no salen nunca al frontend.

use serde::{Deserialize, Serialize};

use super::ProviderError;

const AUTHORIZE_URL: &str = "https://freesound.org/apiv2/oauth2/authorize/";
const TOKEN_URL: &str = "https://freesound.org/apiv2/oauth2/access_token/";

/// Margen con el que se considera que un token esta por vencer.
///
/// Un token que vence en treinta segundos no sirve para empezar una descarga
/// que puede tardar mas que eso.
const EXPIRY_MARGIN_SECONDS: i64 = 120;

/// Tokens de una sesion OAuth2 ya establecida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Momento de vencimiento en segundos desde el epoch UTC.
    pub expires_at: i64,
}

impl OAuthTokens {
    /// Si el token sigue siendo usable dentro del margen de seguridad.
    pub fn is_fresh_at(&self, now: i64) -> bool {
        self.expires_at - EXPIRY_MARGIN_SECONDS > now
    }

    pub fn is_fresh(&self) -> bool {
        self.is_fresh_at(now_seconds())
    }
}

pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// URL a la que hay que mandar al usuario para que autorice la aplicacion.
///
/// `state` no protege gran cosa en un flujo donde el codigo se pega a mano,
/// pero Freesound lo devuelve y permite detectar que el codigo pegado venga de
/// la vuelta que iniciamos nosotros.
pub fn authorize_url(client_id: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE_URL}?client_id={}&response_type=code&state={}",
        super::urlencode(client_id),
        super::urlencode(state)
    )
}

/// Respuesta del endpoint de token. Freesound informa la duracion en segundos.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Duracion que se asume cuando el servidor no informa `expires_in`.
/// Freesound documenta 24 horas.
const DEFAULT_EXPIRES_IN: i64 = 24 * 60 * 60;

/// Canjea el codigo de autorizacion por un par de tokens.
pub async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<OAuthTokens, ProviderError> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "authorization_code"),
        ("code", code.trim()),
    ];
    post_token(client, &form, None).await
}

/// Renueva el acceso con el refresh token.
///
/// Freesound puede devolver un refresh token nuevo o no devolverlo; si no
/// viene, se conserva el que ya teniamos.
pub async fn refresh(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    tokens: &OAuthTokens,
) -> Result<OAuthTokens, ProviderError> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", tokens.refresh_token.as_str()),
    ];
    post_token(client, &form, Some(&tokens.refresh_token)).await
}

async fn post_token(
    client: &reqwest::Client,
    form: &[(&str, &str); 4],
    fallback_refresh: Option<&str>,
) -> Result<OAuthTokens, ProviderError> {
    let response = client
        .post(TOKEN_URL)
        .form(form)
        .send()
        .await
        .map_err(|error| ProviderError::Network(error.to_string()))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        // El cuerpo puede traer el detalle del rechazo, pero tambien puede
        // traer el secreto de vuelta: no se registra crudo.
        return Err(ProviderError::from_status(status.as_u16(), "").into_oauth(status.as_u16()));
    }

    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|error| ProviderError::Parse(format!("respuesta de token: {error}")))?;

    let refresh_token = parsed
        .refresh_token
        .or_else(|| fallback_refresh.map(str::to_string))
        .ok_or_else(|| {
            ProviderError::Parse("la respuesta de token no incluyo refresh_token".into())
        })?;

    Ok(OAuthTokens {
        access_token: parsed.access_token,
        refresh_token,
        expires_at: now_seconds() + parsed.expires_in.unwrap_or(DEFAULT_EXPIRES_IN),
    })
}

impl ProviderError {
    /// Un 400 del endpoint de token casi siempre es un codigo vencido o ya
    /// usado, no una credencial mal puesta: conviene decirlo distinto.
    fn into_oauth(self, status: u16) -> ProviderError {
        match status {
            400 => ProviderError::Unauthorized,
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_la_url_de_autorizacion_escapando_los_parametros() {
        let url = authorize_url("mi id/raro", "estado 1");
        assert!(url.starts_with(AUTHORIZE_URL), "{url}");
        assert!(url.contains("client_id=mi%20id%2Fraro"), "{url}");
        assert!(url.contains("response_type=code"), "{url}");
        assert!(url.contains("state=estado%201"), "{url}");
    }

    #[test]
    fn un_token_vencido_no_se_considera_fresco() {
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: 1_000,
        };

        assert!(tokens.is_fresh_at(0));
        // Justo antes del margen ya se considera vencido, para no arrancar una
        // descarga con un token que muere a mitad de camino.
        assert!(!tokens.is_fresh_at(1_000 - EXPIRY_MARGIN_SECONDS));
        assert!(!tokens.is_fresh_at(2_000));
    }
}
