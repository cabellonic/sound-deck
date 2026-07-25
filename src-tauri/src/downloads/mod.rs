//! Descarga de archivos remotos con limites y validacion (§14).

pub mod url;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::errors::{AppError, AppResult, ErrorKind};
use crate::filesystem::audio_file::{format_bytes, is_acceptable_content_type};

/// User-Agent claro, como pide §13. Identifica la aplicacion y su version.
pub const USER_AGENT: &str = concat!(
    "SoundDeck/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/sound-deck)"
);

/// Cliente HTTP compartido, con timeouts y limite de redirecciones.
pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        // Sin esto una cadena de redirecciones puede volverse infinita (§30).
        .redirect(reqwest::redirect::Policy::limited(5))
        // No guardamos cookies: no hay sesion que mantener con los proveedores.
        .build()
        .unwrap_or_else(|error| {
            tracing::error!(%error, "no se pudo construir el cliente HTTP; se usa el predeterminado");
            reqwest::Client::new()
        })
}

/// Resultado de una descarga a un archivo temporal.
#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub bytes: u64,
    pub content_type: Option<String>,
}

/// Reporte de progreso. Se invoca ya limitado en frecuencia (§24).
pub type ProgressCallback<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

/// Descarga una URL a un archivo temporal aplicando el limite de tamano.
///
/// El limite se verifica dos veces: contra `Content-Length` (cuando existe) y
/// mientras se escribe, porque un servidor puede mentir o no declararlo (§39).
/// Si algo falla, el temporal se borra: no dejamos basura a medio bajar.
pub async fn download_to_temp(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    max_bytes: u64,
    on_progress: Option<ProgressCallback<'_>>,
) -> AppResult<DownloadOutcome> {
    download_to_temp_with_headers(client, url, &[], destination, max_bytes, on_progress).await
}

/// Igual que `download_to_temp`, pero con cabeceras propias de la peticion.
///
/// Existe para el `Authorization` de OAuth2. Las cabeceras se mandan solo al
/// host original: `reqwest` no las reenvia si hay un redirect a otro dominio,
/// que es justo lo que queremos cuando el servidor manda a un CDN firmado.
pub async fn download_to_temp_with_headers(
    client: &reqwest::Client,
    url: &str,
    headers: &[(String, String)],
    destination: &Path,
    max_bytes: u64,
    on_progress: Option<ProgressCallback<'_>>,
) -> AppResult<DownloadOutcome> {
    let mut request = client.get(url);
    for (name, value) in headers {
        request = request.header(name, value);
    }

    let response = request.send().await.map_err(|error| {
        let message = if error.is_timeout() {
            "La descarga tardo demasiado y se cancelo."
        } else if error.is_connect() {
            "No se pudo conectar con el servidor de descarga. Revisa tu conexion."
        } else {
            "No se pudo iniciar la descarga."
        };
        AppError::new(ErrorKind::Download, message).with_technical(error.to_string())
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::new(
            ErrorKind::Download,
            format!(
                "El servidor rechazo la descarga (codigo {}).",
                status.as_u16()
            ),
        )
        .with_technical(format!("GET {url} -> {status}")));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    if let Some(content_type) = &content_type {
        if !is_acceptable_content_type(content_type) {
            return Err(AppError::new(
                ErrorKind::InvalidAudio,
                "El servidor no devolvio un archivo de audio.",
            )
            .with_technical(format!("content-type inesperado: {content_type}")));
        }
    }

    let total_bytes = response.content_length();
    if let Some(total) = total_bytes {
        if total > max_bytes {
            return Err(AppError::new(
                ErrorKind::Download,
                format!(
                    "El archivo pesa {} y supera el limite de {}.",
                    format_bytes(total),
                    format_bytes(max_bytes)
                ),
            ));
        }
    }

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let result = stream_to_file(response, destination, max_bytes, total_bytes, on_progress).await;

    match result {
        Ok(bytes) => Ok(DownloadOutcome {
            path: destination.to_path_buf(),
            bytes,
            content_type,
        }),
        Err(error) => {
            // Limpieza: un temporal incompleto no debe sobrevivir al fallo.
            if let Err(cleanup_error) = tokio::fs::remove_file(destination).await {
                tracing::debug!(
                    path = %destination.display(),
                    %cleanup_error,
                    "no se pudo borrar el temporal fallido"
                );
            }
            Err(error)
        }
    }
}

async fn stream_to_file(
    response: reqwest::Response,
    destination: &Path,
    max_bytes: u64,
    total_bytes: Option<u64>,
    on_progress: Option<ProgressCallback<'_>>,
) -> AppResult<u64> {
    let mut file = tokio::fs::File::create(destination).await?;
    let mut stream = response.bytes_stream();
    let mut written: u64 = 0;

    // Limitamos las notificaciones de progreso: emitir por cada chunk saturaria
    // el IPC sin aportar nada visual (§24).
    let mut last_report = Instant::now();
    const REPORT_INTERVAL: Duration = Duration::from_millis(150);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            AppError::new(
                ErrorKind::Download,
                "La descarga se interrumpio antes de terminar.",
            )
            .with_technical(error.to_string())
        })?;

        written += chunk.len() as u64;
        if written > max_bytes {
            return Err(AppError::new(
                ErrorKind::Download,
                format!(
                    "La descarga supero el limite de {} y se cancelo.",
                    format_bytes(max_bytes)
                ),
            ));
        }

        file.write_all(&chunk).await?;

        if let Some(on_progress) = on_progress {
            if last_report.elapsed() >= REPORT_INTERVAL {
                on_progress(written, total_bytes);
                last_report = Instant::now();
            }
        }
    }

    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    if written == 0 {
        return Err(AppError::new(
            ErrorKind::Download,
            "El servidor devolvio un archivo vacio.",
        ));
    }

    // Reporte final, siempre, para que la barra llegue al 100 %.
    if let Some(on_progress) = on_progress {
        on_progress(written, total_bytes.or(Some(written)));
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// Servidor HTTP local minimo para probar la descarga sin tocar Internet.
    struct TestServer {
        port: u16,
        handle: Option<std::thread::JoinHandle<()>>,
        server: Arc<tiny_http::Server>,
    }

    impl TestServer {
        fn start(body: Vec<u8>, content_type: &str, chunks: usize) -> Self {
            let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
            let port = server.server_addr().to_ip().unwrap().port();
            let content_type = content_type.to_string();
            let thread_server = server.clone();

            let handle = std::thread::spawn(move || {
                while let Ok(request) = thread_server.recv() {
                    let header = tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        content_type.as_bytes(),
                    )
                    .unwrap();
                    let length = body.len();
                    let response = tiny_http::Response::new(
                        tiny_http::StatusCode(200),
                        vec![header],
                        Cursor::new(body.clone()),
                        Some(length),
                        None,
                    );
                    let _ = request.respond(response);
                    if chunks == 0 {
                        break;
                    }
                }
            });

            Self {
                port,
                handle: Some(handle),
                server,
            }
        }

        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{path}", self.port)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.server.unblock();
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn wav_de_prueba() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36u32 + 800).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&8000u32.to_le_bytes());
        out.extend_from_slice(&16000u32.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&800u32.to_le_bytes());
        out.extend(std::iter::repeat_n(0u8, 800));
        out
    }

    /// El `Authorization` de OAuth2 tiene que llegar al servidor: sin el,
    /// Freesound devuelve la pagina de login en vez del archivo original.
    #[tokio::test]
    async fn manda_las_cabeceras_de_autorizacion() {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let visto = Arc::new(std::sync::Mutex::new(None::<String>));

        let thread_server = server.clone();
        let capturado = visto.clone();
        let body = wav_de_prueba();
        let handle = std::thread::spawn(move || {
            if let Ok(request) = thread_server.recv() {
                *capturado.lock().unwrap() = request
                    .headers()
                    .iter()
                    .find(|header| header.field.equiv("Authorization"))
                    .map(|header| header.value.as_str().to_string());

                let length = body.len();
                let _ = request.respond(tiny_http::Response::new(
                    tiny_http::StatusCode(200),
                    vec![
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"audio/wav"[..])
                            .unwrap(),
                    ],
                    Cursor::new(body.clone()),
                    Some(length),
                    None,
                ));
            }
        });

        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("original.wav");
        let headers = vec![("Authorization".to_string(), "Bearer un-token".to_string())];

        download_to_temp_with_headers(
            &build_http_client(),
            &format!("http://127.0.0.1:{port}/sounds/1/download/"),
            &headers,
            &destino,
            10 * 1024 * 1024,
            None,
        )
        .await
        .unwrap();

        server.unblock();
        let _ = handle.join();

        assert_eq!(visto.lock().unwrap().as_deref(), Some("Bearer un-token"));
        assert!(destino.is_file());
    }

    /// Una descarga sin cabeceras no debe inventar ninguna.
    #[tokio::test]
    async fn sin_cabeceras_no_manda_autorizacion() {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let visto = Arc::new(std::sync::Mutex::new(Some("centinela".to_string())));

        let thread_server = server.clone();
        let capturado = visto.clone();
        let body = wav_de_prueba();
        let handle = std::thread::spawn(move || {
            if let Ok(request) = thread_server.recv() {
                *capturado.lock().unwrap() = request
                    .headers()
                    .iter()
                    .find(|header| header.field.equiv("Authorization"))
                    .map(|header| header.value.as_str().to_string());

                let length = body.len();
                let _ = request.respond(tiny_http::Response::new(
                    tiny_http::StatusCode(200),
                    vec![
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"audio/wav"[..])
                            .unwrap(),
                    ],
                    Cursor::new(body.clone()),
                    Some(length),
                    None,
                ));
            }
        });

        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("preview.wav");

        download_to_temp(
            &build_http_client(),
            &format!("http://127.0.0.1:{port}/preview.mp3"),
            &destino,
            10 * 1024 * 1024,
            None,
        )
        .await
        .unwrap();

        server.unblock();
        let _ = handle.join();

        assert_eq!(*visto.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn descarga_un_audio_y_reporta_progreso() {
        let body = wav_de_prueba();
        let esperado = body.len() as u64;
        let server = TestServer::start(body, "audio/wav", 1);
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("descarga.wav");

        let recibido = Arc::new(AtomicU64::new(0));
        let contador = recibido.clone();
        let callback = move |bytes: u64, _total: Option<u64>| {
            contador.store(bytes, Ordering::SeqCst);
        };

        let outcome = download_to_temp(
            &build_http_client(),
            &server.url("/audio.wav"),
            &destino,
            10 * 1024 * 1024,
            Some(&callback),
        )
        .await
        .unwrap();

        assert_eq!(outcome.bytes, esperado);
        assert_eq!(outcome.content_type.as_deref(), Some("audio/wav"));
        assert_eq!(std::fs::metadata(&destino).unwrap().len(), esperado);
        assert_eq!(recibido.load(Ordering::SeqCst), esperado);
    }

    #[tokio::test]
    async fn rechaza_un_content_type_que_no_es_audio() {
        let server = TestServer::start(b"<html>404</html>".to_vec(), "text/html", 1);
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("pagina.mp3");

        let error = download_to_temp(
            &build_http_client(),
            &server.url("/pagina"),
            &destino,
            10 * 1024 * 1024,
            None,
        )
        .await
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidAudio);
        assert!(!destino.exists(), "no debe quedar el temporal");
    }

    #[tokio::test]
    async fn corta_y_limpia_cuando_se_supera_el_limite() {
        let server = TestServer::start(wav_de_prueba(), "audio/wav", 1);
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("grande.wav");

        let error = download_to_temp(
            &build_http_client(),
            &server.url("/audio.wav"),
            &destino,
            100,
            None,
        )
        .await
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::Download);
        assert!(error.message.contains("limite"));
        assert!(!destino.exists(), "el temporal fallido debe borrarse");
    }

    #[tokio::test]
    async fn un_cuerpo_vacio_es_un_error() {
        let server = TestServer::start(Vec::new(), "audio/mpeg", 1);
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("vacio.mp3");

        let error = download_to_temp(
            &build_http_client(),
            &server.url("/vacio"),
            &destino,
            1024,
            None,
        )
        .await
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::Download);
        assert!(!destino.exists());
    }

    #[test]
    fn el_user_agent_identifica_la_aplicacion() {
        assert!(USER_AGENT.starts_with("SoundDeck/"));
        assert!(USER_AGENT.contains(env!("CARGO_PKG_VERSION")));
    }
}
