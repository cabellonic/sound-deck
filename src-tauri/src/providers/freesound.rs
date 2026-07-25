//! Proveedor Freesound (<https://freesound.org>), API v2 documentada y publica.
//!
//! ## Que se descarga
//!
//! Con la sola API key, la API entrega la busqueda y las URLs de *preview* en
//! el CDN: es lo que se guarda si el usuario no conecto su cuenta, y esta
//! cubierto por la misma licencia del sonido. Conectando la cuenta por OAuth2
//! (ver `providers::oauth`) se descarga el archivo **original**, en el formato
//! y la calidad con que se subio.
//!
//! La API key nunca se hardcodea, no aparece en logs y viaja en un header, no
//! en la query string.

use async_trait::async_trait;
use serde::Deserialize;

use crate::domain::category::map_provider_category;
use crate::domain::SoundLicense;

use super::{
    urlencode, ProviderCapabilities, ProviderContext, ProviderError, RemoteSound, ResolvedDownload,
    SearchOptions, SearchPage, SoundProvider,
};

pub const PROVIDER_ID: &str = "freesound";
const API_BASE: &str = "https://freesound.org/apiv2";
const ALLOWED_HOSTS: [&str; 3] = ["freesound.org", "cdn.freesound.org", "freesound-cdn.com"];
const FIELDS: &str = "id,name,description,duration,filesize,tags,license,url,username,previews";

pub struct FreesoundProvider {
    client: reqwest::Client,
}

impl FreesoundProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    count: Option<u64>,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    filesize: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    previews: Option<Previews>,
}

#[derive(Debug, Deserialize)]
struct Previews {
    #[serde(rename = "preview-hq-mp3", default)]
    hq_mp3: Option<String>,
    #[serde(rename = "preview-lq-mp3", default)]
    lq_mp3: Option<String>,
}

impl Previews {
    fn best(&self) -> Option<&str> {
        self.hq_mp3.as_deref().or(self.lq_mp3.as_deref())
    }
}

/// Traduce la URL de licencia de Freesound a un codigo estable.
pub fn parse_license(url: &str) -> Option<SoundLicense> {
    let normalized = url.trim().trim_end_matches('/').to_ascii_lowercase();

    let (code, name) = if normalized.contains("publicdomain/zero") {
        ("cc0-1.0", "CC0 1.0 (dominio publico)")
    } else if normalized.contains("licenses/by-nc") {
        ("cc-by-nc", "Creative Commons Atribucion-NoComercial")
    } else if normalized.contains("licenses/by") {
        ("cc-by", "Creative Commons Atribucion")
    } else if normalized.contains("sampling+") || normalized.contains("sampling%2b") {
        ("sampling-plus", "Creative Commons Sampling Plus")
    } else {
        return None;
    };

    Some(SoundLicense {
        code: code.to_string(),
        name: name.to_string(),
        url: Some(url.to_string()),
    })
}

/// Convierte un resultado crudo de la API al modelo unificado.
///
/// Es una funcion libre para poder probarla contra fixtures sin tocar la red.
fn to_remote_sound(result: SearchResult) -> RemoteSound {
    let title = result
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Sonido {}", result.id));

    let license = result.license.as_deref().and_then(parse_license);

    let attribution = result.username.as_ref().map(|author| {
        let licencia = license
            .as_ref()
            .map(|license| format!(" ({})", license.name))
            .unwrap_or_default();
        format!("\u{201c}{title}\u{201d} por {author} en Freesound{licencia}")
    });

    // La categoria sale de las etiquetas, con las mismas reglas deterministas
    // que el resto de la aplicacion. Si no hay evidencia, no inventamos.
    let normalized_category = map_provider_category(&result.tags.join(" "));

    RemoteSound {
        provider_id: PROVIDER_ID.to_string(),
        remote_id: result.id.to_string(),
        duration_ms: result
            .duration
            .filter(|duration| duration.is_finite() && *duration >= 0.0)
            .map(|duration| (duration * 1000.0) as u64),
        preview_url: result
            .previews
            .as_ref()
            .and_then(Previews::best)
            .map(str::to_string),
        source_page_url: result.url,
        // Guardamos el id, no una URL que pueda caducar (§21).
        download_reference: Some(result.id.to_string()),
        provider_category: None,
        normalized_category,
        tags: result.tags,
        license,
        attribution,
        file_size_bytes: result.filesize,
        description: result
            .description
            .map(|text| text.chars().take(400).collect()),
        title,
    }
}

impl FreesoundProvider {
    fn api_key(context: &ProviderContext) -> Result<&str, ProviderError> {
        context
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or(ProviderError::NotConfigured)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        api_key: &str,
    ) -> Result<T, ProviderError> {
        let response = self
            .client
            .get(url)
            // La clave viaja en el header, nunca en la URL que podria loguearse.
            .header("Authorization", format!("Token {api_key}"))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Unavailable("timeout".into())
                } else {
                    ProviderError::Network(error.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::from_status(status.as_u16(), &body));
        }

        response
            .json::<T>()
            .await
            .map_err(|error| ProviderError::Parse(error.to_string()))
    }
}

#[async_trait]
impl SoundProvider for FreesoundProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn display_name(&self) -> &'static str {
        "Freesound"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            search: true,
            preview: true,
            download: true,
            pagination: true,
            requires_api_key: true,
            unofficial: false,
            oauth: true,
        }
    }

    fn homepage(&self) -> &'static str {
        "https://freesound.org/help/tos_api/"
    }

    fn allowed_hosts(&self) -> &'static [&'static str] {
        &ALLOWED_HOSTS
    }

    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
        context: &ProviderContext,
    ) -> Result<SearchPage, ProviderError> {
        let api_key = Self::api_key(context)?;
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchPage::empty(options.page));
        }

        let url = format!(
            "{API_BASE}/search/text/?query={}&fields={FIELDS}&page={}&page_size={}",
            urlencode(query),
            options.page.max(1),
            options.page_size.clamp(1, 100),
        );

        let response: SearchResponse = self.get_json(&url, api_key).await?;

        Ok(SearchPage {
            page: options.page.max(1),
            has_more: response.next.is_some(),
            total: response.count,
            items: response.results.into_iter().map(to_remote_sound).collect(),
        })
    }

    async fn resolve_download(
        &self,
        item: &RemoteSound,
        context: &ProviderContext,
    ) -> Result<ResolvedDownload, ProviderError> {
        let api_key = Self::api_key(context)?;

        // Volvemos a resolver contra la API en vez de confiar en la URL que
        // teniamos en memoria: puede haber cambiado o caducado (§21).
        let url = format!(
            "{API_BASE}/sounds/{}/?fields={FIELDS}",
            urlencode(&item.remote_id)
        );
        let result: SearchResult = self.get_json(&url, api_key).await?;
        let resolved = to_remote_sound(result);

        let hosts: Vec<String> = ALLOWED_HOSTS.iter().map(|host| host.to_string()).collect();

        // Con la cuenta conectada bajamos el archivo original, en su formato y
        // calidad de subida. Sin conectar queda la preview MP3, que es lo unico
        // que la API entrega con la sola API key.
        if let Some(token) = &context.access_token {
            return Ok(ResolvedDownload {
                url: format!("{API_BASE}/sounds/{}/download/", urlencode(&item.remote_id)),
                allowed_hosts: hosts,
                // El original puede ser wav, aiff, flac u ogg: lo decide el
                // sniffing del contenido, no una extension inventada aca.
                suggested_extension: None,
                license: resolved.license,
                attribution: resolved.attribution,
                headers: vec![("Authorization".to_string(), format!("Bearer {token}"))],
            });
        }

        let download_url = resolved.preview_url.ok_or_else(|| {
            ProviderError::Unavailable(
                "el sonido no tiene una version descargable disponible".into(),
            )
        })?;

        Ok(ResolvedDownload {
            url: download_url,
            allowed_hosts: hosts,
            suggested_extension: Some("mp3".to_string()),
            license: resolved.license,
            attribution: resolved.attribution,
            headers: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture recortado de una respuesta real de la API v2 de Freesound.
    const FIXTURE: &str = include_str!("../../tests/fixtures/freesound_search.json");

    fn parse_fixture() -> SearchResponse {
        serde_json::from_str(FIXTURE).expect("el fixture debe parsear")
    }

    #[test]
    fn parsea_la_respuesta_de_busqueda() {
        let response = parse_fixture();
        assert_eq!(response.count, Some(1523));
        assert!(response.next.is_some());
        assert_eq!(response.results.len(), 3);
    }

    #[test]
    fn mapea_un_resultado_completo() {
        let response = parse_fixture();
        let sonido = to_remote_sound(response.results.into_iter().next().unwrap());

        assert_eq!(sonido.provider_id, "freesound");
        assert_eq!(sonido.remote_id, "573661");
        assert_eq!(sonido.title, "bruh sound effect");
        assert_eq!(sonido.duration_ms, Some(1234));
        assert_eq!(
            sonido.preview_url.as_deref(),
            Some("https://cdn.freesound.org/previews/573/573661_11861866-hq.mp3")
        );
        assert_eq!(sonido.download_reference.as_deref(), Some("573661"));
        assert_eq!(sonido.file_size_bytes, Some(48213));
        assert_eq!(sonido.tags, vec!["meme", "voice", "bruh"]);

        let licencia = sonido.license.expect("tiene licencia");
        assert_eq!(licencia.code, "cc0-1.0");

        let atribucion = sonido.attribution.expect("tiene atribucion");
        assert!(atribucion.contains("usuario-demo"));
        assert!(atribucion.contains("Freesound"));
    }

    #[test]
    fn infiere_la_categoria_desde_las_etiquetas() {
        let response = parse_fixture();
        let sonidos: Vec<RemoteSound> = response.results.into_iter().map(to_remote_sound).collect();

        assert_eq!(
            sonidos[0].normalized_category,
            Some(crate::domain::NormalizedCategory::Memes)
        );
        // El tercero no tiene etiquetas reconocibles: no inventamos categoria.
        assert_eq!(sonidos[2].normalized_category, None);
    }

    #[test]
    fn tolera_campos_faltantes_sin_romperse() {
        let response = parse_fixture();
        let minimo = to_remote_sound(response.results.into_iter().nth(2).unwrap());

        assert_eq!(minimo.remote_id, "999");
        // Sin `name` usable, el titulo cae a un valor deterministico.
        assert_eq!(minimo.title, "Sonido 999");
        assert_eq!(minimo.duration_ms, None);
        assert_eq!(minimo.preview_url, None);
        assert!(minimo.license.is_none());
    }

    #[test]
    fn usa_la_preview_de_baja_calidad_si_no_hay_alta() {
        let response = parse_fixture();
        let segundo = to_remote_sound(response.results.into_iter().nth(1).unwrap());
        assert_eq!(
            segundo.preview_url.as_deref(),
            Some("https://cdn.freesound.org/previews/100/100_1-lq.mp3")
        );
    }

    #[test]
    fn mapea_las_licencias_conocidas() {
        assert_eq!(
            parse_license("https://creativecommons.org/publicdomain/zero/1.0/")
                .unwrap()
                .code,
            "cc0-1.0"
        );
        assert_eq!(
            parse_license("http://creativecommons.org/licenses/by/4.0/")
                .unwrap()
                .code,
            "cc-by"
        );
        assert_eq!(
            parse_license("https://creativecommons.org/licenses/by-nc/3.0/")
                .unwrap()
                .code,
            "cc-by-nc"
        );
        assert!(parse_license("https://ejemplo.com/licencia-rara").is_none());
    }

    #[test]
    fn una_respuesta_corrupta_no_entra_en_panico() {
        assert!(serde_json::from_str::<SearchResponse>("{}").is_ok());
        assert!(serde_json::from_str::<SearchResponse>("no es json").is_err());
        assert!(serde_json::from_str::<SearchResponse>(r#"{"results":[]}"#).is_ok());
    }

    #[tokio::test]
    async fn sin_api_key_falla_antes_de_tocar_la_red() {
        let provider = FreesoundProvider::new(reqwest::Client::new());
        let error = provider
            .search(
                "bruh",
                SearchOptions::default(),
                &ProviderContext::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, ProviderError::NotConfigured));
    }

    #[tokio::test]
    async fn una_busqueda_vacia_no_consulta_al_proveedor() {
        let provider = FreesoundProvider::new(reqwest::Client::new());
        let context = ProviderContext {
            api_key: Some("clave-de-prueba".into()),
            ..Default::default()
        };

        let page = provider
            .search("   ", SearchOptions::default(), &context)
            .await
            .unwrap();
        assert!(page.items.is_empty());
        assert!(!page.has_more);
    }
}
