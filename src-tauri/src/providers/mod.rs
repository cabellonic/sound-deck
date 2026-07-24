//! Abstraccion de proveedores de sonidos online (§11, §12).
//!
//! El modelo de datos de la aplicacion no depende de ningun proveedor concreto:
//! cada uno traduce su respuesta a `RemoteSound`. Agregar un proveedor nuevo es
//! implementar el trait y registrarlo.

pub mod freesound;
pub mod myinstants;
pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::{NormalizedCategory, SoundLicense};

/// Lo que un proveedor sabe hacer. La interfaz lo usa para no ofrecer acciones
/// imposibles (por ejemplo, un boton de preview en un proveedor sin previews).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub search: bool,
    pub preview: bool,
    pub download: bool,
    pub pagination: bool,
    /// Si necesita una API key configurada para funcionar.
    pub requires_api_key: bool,
    /// Proveedor no oficial (sin API publica documentada). Se marca en la UI (§12).
    pub unofficial: bool,
}

/// Resultado unificado de una busqueda online.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSound {
    pub provider_id: String,
    pub remote_id: String,
    pub title: String,
    pub description: Option<String>,
    pub duration_ms: Option<u64>,
    /// URL para escuchar sin descargar. Puede caducar: no se persiste.
    pub preview_url: Option<String>,
    pub source_page_url: Option<String>,
    /// Referencia estable para volver a resolver la descarga con el proveedor.
    pub download_reference: Option<String>,
    pub provider_category: Option<String>,
    pub normalized_category: Option<NormalizedCategory>,
    pub tags: Vec<String>,
    pub license: Option<SoundLicense>,
    pub attribution: Option<String>,
    /// Tamano declarado por el proveedor, si lo informa.
    pub file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub page: u32,
    pub page_size: u32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 30,
        }
    }
}

/// Una pagina de resultados.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub items: Vec<RemoteSound>,
    pub page: u32,
    pub has_more: bool,
    pub total: Option<u64>,
}

impl SearchPage {
    pub fn empty(page: u32) -> Self {
        Self {
            items: Vec::new(),
            page,
            has_more: false,
            total: Some(0),
        }
    }
}

/// Descarga resuelta y lista para bajarse.
#[derive(Debug, Clone)]
pub struct ResolvedDownload {
    pub url: String,
    /// Hosts permitidos para esta descarga. La validacion los exige (§30).
    pub allowed_hosts: Vec<String>,
    pub suggested_extension: Option<String>,
    pub license: Option<SoundLicense>,
    pub attribution: Option<String>,
}

/// Errores que puede reportar un proveedor. Se traducen a mensajes accionables.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("el proveedor no esta configurado")]
    NotConfigured,
    #[error("credenciales invalidas")]
    Unauthorized,
    #[error("limite de solicitudes alcanzado")]
    RateLimited,
    #[error("error de red: {0}")]
    Network(String),
    #[error("respuesta inesperada: {0}")]
    Parse(String),
    #[error("el proveedor no esta disponible: {0}")]
    Unavailable(String),
    #[error("operacion no soportada por este proveedor")]
    Unsupported,
}

impl ProviderError {
    /// Mensaje comprensible para el usuario, con el nombre del proveedor.
    pub fn user_message(&self, provider_name: &str) -> String {
        match self {
            ProviderError::NotConfigured => format!(
                "{provider_name} necesita una API key. Configurala en Ajustes > Proveedores."
            ),
            ProviderError::Unauthorized => format!(
                "La API key de {provider_name} no es valida o expiro. Revisala en Ajustes > Proveedores."
            ),
            ProviderError::RateLimited => format!(
                "{provider_name} rechazo la consulta por exceso de solicitudes. Esperá unos minutos."
            ),
            ProviderError::Network(_) => format!(
                "No se pudo contactar a {provider_name}. Revisa tu conexion a Internet."
            ),
            ProviderError::Parse(_) => format!(
                "{provider_name} respondio en un formato inesperado. Puede haber cambiado su servicio."
            ),
            ProviderError::Unavailable(_) => {
                format!("{provider_name} no esta disponible en este momento.")
            }
            ProviderError::Unsupported => {
                format!("{provider_name} no permite esta operacion.")
            }
        }
    }

    pub fn from_status(status: u16, body: &str) -> Self {
        match status {
            401 | 403 => ProviderError::Unauthorized,
            429 => ProviderError::RateLimited,
            400..=499 => ProviderError::Unavailable(format!("HTTP {status}")),
            500..=599 => ProviderError::Unavailable(format!("HTTP {status}")),
            _ => ProviderError::Parse(format!("HTTP {status}: {}", truncate(body, 200))),
        }
    }
}

/// Percent-encoding minimo para query strings.
///
/// Es lo unico que necesitamos de una libreria de URLs completa, y ademas
/// neutraliza cualquier intento de inyectar parametros extra en la consulta.
pub(crate) fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect::<String>() + "..."
    }
}

/// Configuracion que la aplicacion entrega al proveedor en cada llamada.
#[derive(Debug, Clone, Default)]
pub struct ProviderContext {
    pub api_key: Option<String>,
}

/// Un proveedor de sonidos online.
#[async_trait]
pub trait SoundProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;
    /// Pagina publica del servicio, para que el usuario revise sus terminos.
    fn homepage(&self) -> &'static str;

    /// Hosts a los que este proveedor puede pedir audio. Toda URL de descarga o
    /// preview se valida contra esta lista antes de tocarla (§30).
    fn allowed_hosts(&self) -> &'static [&'static str];

    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
        context: &ProviderContext,
    ) -> Result<SearchPage, ProviderError>;

    async fn resolve_download(
        &self,
        item: &RemoteSound,
        context: &ProviderContext,
    ) -> Result<ResolvedDownload, ProviderError>;

    /// Comprobacion ligera de que las credenciales funcionan (§20 "probar conexion").
    async fn test_connection(&self, context: &ProviderContext) -> Result<(), ProviderError> {
        self.search(
            "test",
            SearchOptions {
                page: 1,
                page_size: 1,
            },
            context,
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_mensajes_de_error_nombran_al_proveedor_y_son_accionables() {
        let mensaje = ProviderError::NotConfigured.user_message("Freesound");
        assert!(mensaje.contains("Freesound"));
        assert!(mensaje.contains("Ajustes"));

        let mensaje = ProviderError::Unauthorized.user_message("Freesound");
        assert!(mensaje.contains("API key"));

        // Nunca filtramos el detalle tecnico crudo en el mensaje al usuario.
        let mensaje =
            ProviderError::Network("dns error: token=abc123".into()).user_message("Freesound");
        assert!(!mensaje.contains("abc123"));
    }

    #[test]
    fn mapea_codigos_http_a_errores_de_dominio() {
        assert!(matches!(
            ProviderError::from_status(401, ""),
            ProviderError::Unauthorized
        ));
        assert!(matches!(
            ProviderError::from_status(429, ""),
            ProviderError::RateLimited
        ));
        assert!(matches!(
            ProviderError::from_status(503, ""),
            ProviderError::Unavailable(_)
        ));
    }

    #[test]
    fn opciones_de_busqueda_por_defecto() {
        let opciones = SearchOptions::default();
        assert_eq!(opciones.page, 1);
        assert_eq!(opciones.page_size, 30);
    }

    #[test]
    fn codifica_la_query_string() {
        assert_eq!(urlencode("risa malvada"), "risa%20malvada");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencode("perro"), "perro");
        // Un intento de inyectar parametros extra queda neutralizado.
        assert!(!urlencode("x&token=robado").contains('&'));
        // Los caracteres no ASCII se codifican byte a byte en UTF-8.
        assert_eq!(urlencode("ñ"), "%C3%B1");
    }
}
