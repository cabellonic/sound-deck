//! Proveedor **no oficial** MyInstants (<https://www.myinstants.com>).
//!
//! ## Por que existe y bajo que condiciones
//!
//! MyInstants no publica una API. Este proveedor lee las mismas paginas
//! publicas de busqueda que veria cualquier persona con un navegador, y solo
//! cuando el usuario escribe algo. Antes de implementarlo se revisaron las
//! restricciones publicas del sitio (§12.1):
//!
//! - Su `robots.txt` declara `Allow: /` para el user-agent generico `*`, y solo
//!   bloquea rutas puntuales (`/add/`, `/report/`, `/analytics/`, `/gifs/`,
//!   `/image/`, `/beyond/`, `/facebook/`) y a una lista de crawlers de
//!   entrenamiento de IA. `/search/` no esta restringido.
//! - Sound Deck se identifica con su propio User-Agent y cae bajo `*`.
//!
//! Reglas que respetamos, en el codigo y no solo en la documentacion:
//!
//! - **Solo bajo demanda**: una consulta por busqueda del usuario. No hay
//!   crawler, ni recorrido del catalogo, ni descarga masiva.
//! - **Rate limit propio**: nunca mas de una peticion cada [`MIN_REQUEST_GAP`].
//! - **Sin evasion**: no tocamos rutas prohibidas, no falsificamos el
//!   User-Agent, no resolvemos CAPTCHAs ni saltamos autenticacion.
//! - **Desactivado por defecto**: hay que habilitarlo a mano en Ajustes.
//! - **Aislado**: si el HTML cambia, falla solo este proveedor con un mensaje
//!   claro; el resto de la aplicacion sigue funcionando.
//!
//! El modelo de datos de la aplicacion no depende de este HTML: como cualquier
//! otro proveedor, traduce a [`RemoteSound`] y ahi termina su alcance.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use scraper::{Html, Selector};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::domain::category::map_provider_category;

use super::{
    urlencode, ProviderCapabilities, ProviderContext, ProviderError, RemoteSound, ResolvedDownload,
    SearchOptions, SearchPage, SoundProvider,
};

pub const PROVIDER_ID: &str = "myinstants";

const BASE_URL: &str = "https://www.myinstants.com";
const ALLOWED_HOSTS: [&str; 2] = ["myinstants.com", "www.myinstants.com"];

/// Espera minima entre dos peticiones al sitio. Es un limite que nos ponemos
/// nosotros, no uno que el sitio nos imponga.
const MIN_REQUEST_GAP: Duration = Duration::from_millis(1200);

/// Cuantos resultados trae una pagina de busqueda del sitio.
///
/// Es una heuristica: el sitio no informa el total ni el tamano de pagina. Si el
/// valor real fuera menor, `has_more` daria siempre `false` y simplemente no se
/// ofreceria "cargar mas". Preferimos quedarnos cortos antes que prometer una
/// pagina siguiente que no existe.
const RESULTS_PER_PAGE: usize = 30;

pub struct MyInstantsProvider {
    client: reqwest::Client,
    /// Momento de la ultima peticion, para espaciar las siguientes.
    last_request: Arc<Mutex<Option<Instant>>>,
}

impl MyInstantsProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    /// Espera lo necesario para no superar nuestro propio limite de frecuencia.
    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;

        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < MIN_REQUEST_GAP {
                tokio::time::sleep(MIN_REQUEST_GAP - elapsed).await;
            }
        }

        *last = Some(Instant::now());
    }

    async fn get_html(&self, url: &str) -> Result<String, ProviderError> {
        self.throttle().await;

        let response = self
            .client
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml")
            .header("Accept-Language", "es,en;q=0.8")
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
            .text()
            .await
            .map_err(|error| ProviderError::Parse(error.to_string()))
    }
}

/// Un resultado extraido del HTML, antes de convertirse en `RemoteSound`.
#[derive(Debug, PartialEq, Eq)]
struct ParsedInstant {
    title: String,
    /// Slug canonico del sonido, tomado del enlace de detalle.
    slug: String,
    /// Ruta del audio, relativa al sitio.
    media_path: String,
    detail_path: Option<String>,
}

/// Extrae los resultados de una pagina de busqueda.
///
/// Devuelve `Err` solo si la pagina no se parece en nada a lo esperado; una
/// busqueda legitima sin resultados devuelve una lista vacia. La diferencia
/// importa: un cambio de estructura debe avisarse, no disfrazarse de "no hay
/// nada" (§12.4).
fn parse_search_page(html: &str) -> Result<Vec<ParsedInstant>, ProviderError> {
    // Los selectores son constantes validas; si fallaran seria un bug nuestro.
    let container = Selector::parse("div.instant").expect("selector valido");
    let link = Selector::parse("a.instant-link").expect("selector valido");
    let button = Selector::parse(".small-button").expect("selector valido");

    let document = Html::parse_document(html);
    let mut results = Vec::new();

    for element in document.select(&container) {
        let anchor = element.select(&link).next();

        let title = anchor
            .map(|node| node.text().collect::<String>())
            .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();

        let detail_path = anchor
            .and_then(|node| node.value().attr("href"))
            .map(str::to_string);

        // El boton lleva la ruta del audio dentro de `play('...')`, en `onclick`
        // o en `onmousedown` segun la version del sitio.
        let media_path = element.select(&button).find_map(|node| {
            node.value()
                .attr("onclick")
                .or_else(|| node.value().attr("onmousedown"))
                .and_then(extract_play_argument)
        });

        let (Some(media_path), false) = (media_path, title.is_empty()) else {
            // Una tarjeta sin audio o sin nombre no sirve: la salteamos en vez
            // de inventar datos.
            continue;
        };

        let slug = detail_path
            .as_deref()
            .and_then(slug_from_detail_path)
            .or_else(|| slug_from_media_path(&media_path))
            .unwrap_or_else(|| media_path.clone());

        results.push(ParsedInstant {
            title,
            slug,
            media_path,
            detail_path,
        });
    }

    if results.is_empty() && !looks_like_a_results_page(html) {
        return Err(ProviderError::Parse(
            "la pagina de busqueda no tiene la estructura esperada".into(),
        ));
    }

    Ok(results)
}

/// Heuristica para distinguir "cero resultados" de "el sitio cambio".
///
/// Una pagina de busqueda real, aunque no encuentre nada, sigue siendo la
/// pagina de MyInstants y menciona sus clases o su formulario de busqueda.
fn looks_like_a_results_page(html: &str) -> bool {
    html.contains("instant") || html.contains("myinstants")
}

/// Saca la ruta del audio de un `onclick="play('/media/sounds/x.mp3', ...)"`.
///
/// Se hace a mano y no con una expresion regular para no arrastrar la
/// dependencia; ademas valida la forma de la ruta, que es lo que importa.
fn extract_play_argument(attribute: &str) -> Option<String> {
    let start = attribute.find("play(")? + "play(".len();
    let rest = attribute.get(start..)?.trim_start();

    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let inner = rest.get(quote.len_utf8()..)?;
    let end = inner.find(quote)?;
    let path = inner.get(..end)?.trim();

    // Solo aceptamos rutas relativas del propio sitio. Nada de esquemas ni de
    // hosts ajenos colandose desde el HTML.
    if path.is_empty() || !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    if path.contains("..") {
        return None;
    }

    Some(path.to_string())
}

/// `/en/instant/bruh-6968/` -> `bruh-6968`
fn slug_from_detail_path(path: &str) -> Option<String> {
    let slug = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|slug| !slug.is_empty())?;
    Some(slug.to_string())
}

/// `/media/sounds/bruh_1.mp3` -> `bruh_1`
fn slug_from_media_path(path: &str) -> Option<String> {
    let file = path.rsplit('/').next()?;
    let stem = file.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file);
    if stem.is_empty() {
        None
    } else {
        Some(stem.to_string())
    }
}

fn absolute_url(path: &str) -> String {
    format!("{BASE_URL}{path}")
}

fn to_remote_sound(parsed: ParsedInstant) -> RemoteSound {
    let media_url = absolute_url(&parsed.media_path);

    RemoteSound {
        provider_id: PROVIDER_ID.to_string(),
        remote_id: parsed.slug,
        // El audio es el mismo archivo para escuchar y para guardar: son
        // clips cortos, no hay una version "de preview" aparte.
        preview_url: Some(media_url.clone()),
        download_reference: Some(parsed.media_path),
        source_page_url: parsed.detail_path.as_deref().map(absolute_url),
        normalized_category: map_provider_category(&parsed.title),
        provider_category: None,
        tags: Vec::new(),
        // El sitio no declara licencia por sonido: no inventamos una.
        license: None,
        attribution: Some(format!(
            "\u{201c}{}\u{201d} desde MyInstants (subido por la comunidad, sin licencia declarada)",
            parsed.title
        )),
        description: None,
        duration_ms: None,
        file_size_bytes: None,
        title: parsed.title,
    }
}

#[async_trait]
impl SoundProvider for MyInstantsProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn display_name(&self) -> &'static str {
        "MyInstants"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            search: true,
            preview: true,
            download: true,
            pagination: true,
            requires_api_key: false,
            // Sin API oficial: la interfaz lo marca y avisa que puede romperse.
            unofficial: true,
            oauth: false,
        }
    }

    fn homepage(&self) -> &'static str {
        "https://www.myinstants.com/en/terms/"
    }

    fn allowed_hosts(&self) -> &'static [&'static str] {
        &ALLOWED_HOSTS
    }

    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
        _context: &ProviderContext,
    ) -> Result<SearchPage, ProviderError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchPage::empty(options.page));
        }

        let page = options.page.max(1);
        let url = format!(
            "{BASE_URL}/en/search/?name={}&page={page}",
            urlencode(query)
        );

        let html = self.get_html(&url).await?;
        let parsed = parse_search_page(&html)?;

        // El sitio no informa el total. Si la pagina vino llena, asumimos que
        // hay mas; si no, cortamos. Nunca pedimos paginas por adelantado.
        let has_more = parsed.len() >= RESULTS_PER_PAGE;

        Ok(SearchPage {
            items: parsed.into_iter().map(to_remote_sound).collect(),
            page,
            has_more,
            total: None,
        })
    }

    async fn resolve_download(
        &self,
        item: &RemoteSound,
        _context: &ProviderContext,
    ) -> Result<ResolvedDownload, ProviderError> {
        // Camino normal: la busqueda ya nos dio la ruta del audio.
        let media_path = match item.download_reference.as_deref() {
            Some(path) if path.starts_with('/') => path.to_string(),
            // Fallback: volvemos a resolverlo desde la pagina de detalle, para
            // que una referencia vieja no deje al sonido inservible (§21).
            _ => {
                let detail = item
                    .source_page_url
                    .clone()
                    .unwrap_or_else(|| format!("{BASE_URL}/en/instant/{}/", item.remote_id));

                let html = self.get_html(&detail).await?;
                parse_search_page(&html)?
                    .into_iter()
                    .next()
                    .map(|parsed| parsed.media_path)
                    .ok_or_else(|| {
                        ProviderError::Unavailable(
                            "el sonido ya no esta disponible en el sitio".into(),
                        )
                    })?
            }
        };

        let extension = media_path
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .filter(|extension| extension.len() <= 5);

        Ok(ResolvedDownload {
            url: absolute_url(&media_path),
            allowed_hosts: ALLOWED_HOSTS.iter().map(|host| host.to_string()).collect(),
            suggested_extension: extension.or_else(|| Some("mp3".to_string())),
            license: None,
            attribution: item.attribution.clone(),
            headers: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture con la forma de una pagina de resultados, incluidos los casos
    /// raros que aparecen en la vida real.
    const FIXTURE: &str = include_str!("../../tests/fixtures/myinstants_search.html");

    fn parse() -> Vec<ParsedInstant> {
        parse_search_page(FIXTURE).expect("el fixture debe parsear")
    }

    #[test]
    fn extrae_los_resultados_bien_formados() {
        let resultados = parse();
        // Cinco tarjetas en el fixture, dos de ellas invalidas a proposito.
        assert_eq!(resultados.len(), 3);

        let primero = &resultados[0];
        assert_eq!(primero.title, "bruh sound effect");
        assert_eq!(primero.slug, "bruh-sound-effect-6968");
        assert_eq!(primero.media_path, "/media/sounds/bruh-sound-effect.mp3");
        assert_eq!(
            primero.detail_path.as_deref(),
            Some("/en/instant/bruh-sound-effect-6968/")
        );
    }

    #[test]
    fn soporta_onmousedown_ademas_de_onclick() {
        let resultados = parse();
        let segundo = &resultados[1];
        assert_eq!(segundo.title, "risa malvada");
        assert_eq!(segundo.media_path, "/media/sounds/risa-malvada.mp3");
    }

    #[test]
    fn normaliza_los_espacios_del_nombre() {
        let resultados = parse();
        let tercero = &resultados[2];
        assert_eq!(tercero.title, "que pasa bebé");
    }

    #[test]
    fn descarta_tarjetas_sin_audio_o_sin_nombre() {
        let resultados = parse();
        assert!(resultados.iter().all(|r| !r.title.is_empty()));
        assert!(resultados.iter().all(|r| r.media_path.starts_with('/')));
        assert!(!resultados.iter().any(|r| r.title == "sin audio"));
    }

    #[test]
    fn convierte_a_remote_sound_con_urls_absolutas() {
        let sonido = to_remote_sound(parse().into_iter().next().unwrap());

        assert_eq!(sonido.provider_id, "myinstants");
        assert_eq!(sonido.remote_id, "bruh-sound-effect-6968");
        assert_eq!(
            sonido.preview_url.as_deref(),
            Some("https://www.myinstants.com/media/sounds/bruh-sound-effect.mp3")
        );
        assert_eq!(
            sonido.source_page_url.as_deref(),
            Some("https://www.myinstants.com/en/instant/bruh-sound-effect-6968/")
        );
        // La referencia guardada es relativa: sobrevive a un cambio de dominio.
        assert_eq!(
            sonido.download_reference.as_deref(),
            Some("/media/sounds/bruh-sound-effect.mp3")
        );
        // Sin licencia declarada, no inventamos una.
        assert!(sonido.license.is_none());
        assert!(sonido.attribution.unwrap().contains("MyInstants"));
    }

    #[test]
    fn infiere_categoria_solo_cuando_el_nombre_lo_permite() {
        let resultados = parse();
        let risa = to_remote_sound(
            resultados
                .into_iter()
                .find(|r| r.title == "risa malvada")
                .unwrap(),
        );
        // "risa malvada" no coincide con ninguna regla: no forzamos categoria.
        assert_eq!(risa.normalized_category, None);
    }

    #[test]
    fn una_busqueda_sin_resultados_no_es_un_error() {
        let vacio = r#"<html><body><div class="search-results">
            <p>No instants found for your search on myinstants.</p>
        </div></body></html>"#;
        assert_eq!(parse_search_page(vacio).unwrap().len(), 0);
    }

    #[test]
    fn un_html_irreconocible_se_reporta_como_cambio_de_estructura() {
        let error = parse_search_page("<html><body><h1>503 Service Unavailable</h1></body></html>")
            .unwrap_err();
        assert!(matches!(error, ProviderError::Parse(_)));

        // Y el mensaje que ve el usuario nombra al proveedor y no filtra HTML.
        let mensaje = error.user_message("MyInstants");
        assert!(mensaje.contains("MyInstants"));
        assert!(!mensaje.contains("503"));
    }

    #[test]
    fn extrae_el_argumento_de_play_en_sus_variantes() {
        assert_eq!(
            extract_play_argument("play('/media/sounds/a.mp3', '/instant/a/', 'a')").as_deref(),
            Some("/media/sounds/a.mp3")
        );
        assert_eq!(
            extract_play_argument("  play(\"/media/sounds/b.mp3\")").as_deref(),
            Some("/media/sounds/b.mp3")
        );
        assert_eq!(
            extract_play_argument("javascript:play( '/media/c.mp3' )").as_deref(),
            Some("/media/c.mp3")
        );
    }

    #[test]
    fn rechaza_rutas_hostiles_incrustadas_en_el_html() {
        // Host ajeno, protocolo relativo, traversal y esquemas raros: todo fuera.
        for hostil in [
            "play('https://malicioso.com/x.mp3')",
            "play('//malicioso.com/x.mp3')",
            "play('/media/../../etc/passwd')",
            "play('file:///C:/Windows/x.mp3')",
            "play('')",
            "reproducir('/media/a.mp3')",
            "play(",
        ] {
            assert_eq!(extract_play_argument(hostil), None, "{hostil}");
        }
    }

    #[test]
    fn deriva_el_slug_del_enlace_o_del_archivo() {
        assert_eq!(
            slug_from_detail_path("/en/instant/bruh-6968/").as_deref(),
            Some("bruh-6968")
        );
        assert_eq!(slug_from_detail_path("/").as_deref(), None);
        assert_eq!(
            slug_from_media_path("/media/sounds/risa_1.mp3").as_deref(),
            Some("risa_1")
        );
    }

    #[tokio::test]
    async fn el_rate_limit_espacia_las_peticiones() {
        let provider = MyInstantsProvider::new(reqwest::Client::new());

        let inicio = std::time::Instant::now();
        provider.throttle().await; // La primera no espera.
        provider.throttle().await; // La segunda si.
        let transcurrido = inicio.elapsed();

        assert!(
            transcurrido >= MIN_REQUEST_GAP,
            "se esperaba al menos {MIN_REQUEST_GAP:?}, paso {transcurrido:?}"
        );
    }

    #[tokio::test]
    async fn una_busqueda_vacia_no_toca_la_red() {
        let provider = MyInstantsProvider::new(reqwest::Client::new());

        let page = provider
            .search("   ", SearchOptions::default(), &ProviderContext::default())
            .await
            .unwrap();

        assert!(page.items.is_empty());
        assert!(!page.has_more);
    }

    #[test]
    fn no_requiere_api_key_y_se_declara_no_oficial() {
        let provider = MyInstantsProvider::new(reqwest::Client::new());
        let capabilities = provider.capabilities();

        assert!(!capabilities.requires_api_key);
        assert!(capabilities.unofficial);
        assert_eq!(
            provider.allowed_hosts(),
            &["myinstants.com", "www.myinstants.com"]
        );
    }
}
