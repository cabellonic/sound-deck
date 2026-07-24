//! Tests de integracion (§34).
//!
//! Cubren el recorrido completo de datos: migraciones, paginas, slots, ingesta
//! de archivos, deduplicacion y descarga contra un servidor HTTP local. Ninguno
//! toca Internet ni necesita una ventana de Tauri.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use sound_deck_lib::database::{pages, slots, sounds, Database};
use sound_deck_lib::domain::{SlotNumber, SoundSource};
use sound_deck_lib::downloads::{build_http_client, download_to_temp};
use sound_deck_lib::filesystem::AppPaths;
use sound_deck_lib::library::{self, IngestOutcome, IngestRequest, SourceHandling};

const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// WAV PCM valido y minimo. `marca` cambia el contenido para variar el hash.
fn wav(millis: u32, marca: u8) -> Vec<u8> {
    let sample_rate = 8000u32;
    let samples = sample_rate * millis / 1000;
    let data_len = samples * 2;

    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend(std::iter::repeat_n(marca, data_len as usize));
    out
}

struct Entorno {
    db: Database,
    paths: AppPaths,
    _data: tempfile::TempDir,
    origen: tempfile::TempDir,
}

impl Entorno {
    fn nuevo() -> Self {
        let data = tempfile::tempdir().expect("directorio de datos");
        let paths = AppPaths::with_root(data.path()).expect("layout");
        let db = Database::open(&paths.database_file()).expect("base de datos");
        db.ensure_initial_state().expect("estado inicial");

        Self {
            db,
            paths,
            _data: data,
            origen: tempfile::tempdir().expect("directorio de origen"),
        }
    }

    fn archivo(&self, nombre: &str, marca: u8) -> PathBuf {
        let ruta = self.origen.path().join(nombre);
        std::fs::write(&ruta, wav(250, marca)).expect("escribir archivo de prueba");
        ruta
    }

    fn importar(&self, nombre: &str, marca: u8) -> IngestOutcome {
        library::ingest(
            &self.db,
            &self.paths,
            IngestRequest::local_import(self.archivo(nombre, marca)),
            MAX_BYTES,
        )
        .expect("importacion")
    }
}

fn slot(n: u8) -> SlotNumber {
    SlotNumber::new(n).expect("numero de slot valido")
}

#[test]
fn primer_arranque_crea_la_pagina_principal() {
    let entorno = Entorno::nuevo();

    let paginas = pages::list_summaries(&entorno.db).unwrap();
    assert_eq!(paginas.len(), 1, "debe existir exactamente una pagina");
    assert_eq!(paginas[0].name, "Principal");
    assert_eq!(paginas[0].position, 0);

    let pagina = pages::first(&entorno.db).unwrap().unwrap();
    assert_eq!(pagina.slots.len(), 9);
    assert!(pagina.slots.iter().all(|slot| slot.sound.is_none()));
}

#[test]
fn los_datos_sobreviven_a_un_reinicio() {
    let data = tempfile::tempdir().unwrap();
    let paths = AppPaths::with_root(data.path()).unwrap();
    let origen = tempfile::tempdir().unwrap();
    let archivo = origen.path().join("audio.wav");
    std::fs::write(&archivo, wav(300, 1)).unwrap();

    let (page_id, sound_id) = {
        let db = Database::open(&paths.database_file()).unwrap();
        db.ensure_initial_state().unwrap();

        let pagina = pages::create(&db, "Discord").unwrap();
        let sonido = library::ingest(
            &db,
            &paths,
            IngestRequest::local_import(archivo.clone()),
            MAX_BYTES,
        )
        .unwrap()
        .sound()
        .clone();

        slots::assign(&db, &pagina.id, slot(1), &sonido.id).unwrap();
        (pagina.id, sonido.id)
    };

    // Segunda "ejecucion" de la aplicacion sobre los mismos datos.
    let db = Database::open(&paths.database_file()).unwrap();
    db.ensure_initial_state().unwrap();

    assert_eq!(pages::list_summaries(&db).unwrap().len(), 2);

    let pagina = pages::get(&db, &page_id)
        .unwrap()
        .expect("la pagina persiste");
    let asignado = pagina.slots[0]
        .sound
        .as_ref()
        .expect("la asignacion persiste");
    assert_eq!(asignado.id, sound_id);
    assert!(asignado.file_available, "el archivo sigue en disco");
}

#[test]
fn importar_deduplica_por_contenido() {
    let entorno = Entorno::nuevo();

    let primero = entorno.importar("original.wav", 5);
    let segundo = entorno.importar("copia.wav", 5);
    let distinto = entorno.importar("otro.wav", 6);

    assert!(!primero.is_duplicate());
    assert!(segundo.is_duplicate());
    assert!(!distinto.is_duplicate());
    assert_eq!(primero.sound().id, segundo.sound().id);

    let archivos = std::fs::read_dir(entorno.paths.sounds_dir())
        .unwrap()
        .count();
    assert_eq!(archivos, 2, "solo dos binarios distintos");
}

#[test]
fn eliminar_un_sonido_usado_limpia_todo_sin_dejar_huerfanos() {
    let entorno = Entorno::nuevo();
    let pagina = pages::first(&entorno.db).unwrap().unwrap();
    let otra = pages::create(&entorno.db, "Juegos").unwrap();
    let sonido = entorno.importar("compartido.wav", 3).sound().clone();

    slots::assign(&entorno.db, &pagina.id, slot(1), &sonido.id).unwrap();
    slots::assign(&entorno.db, &otra.id, slot(5), &sonido.id).unwrap();

    let uso = sounds::usage(&entorno.db, &sonido.id).unwrap();
    assert_eq!(uso.len(), 2, "el uso se reporta antes de confirmar");
    assert!(uso
        .iter()
        .any(|u| u.page_name == "Principal" && u.slot_number == 1));
    assert!(uso
        .iter()
        .any(|u| u.page_name == "Juegos" && u.slot_number == 5));

    library::delete_sound(&entorno.db, &entorno.paths, &sonido.id).unwrap();

    assert!(sounds::find_by_id(&entorno.db, &sonido.id)
        .unwrap()
        .is_none());
    assert!(slots::get(&entorno.db, &pagina.id, slot(1))
        .unwrap()
        .unwrap()
        .sound
        .is_none());
    assert!(slots::get(&entorno.db, &otra.id, slot(5))
        .unwrap()
        .unwrap()
        .sound
        .is_none());
    assert_eq!(
        std::fs::read_dir(entorno.paths.sounds_dir())
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn borrar_una_pagina_no_borra_los_audios() {
    let entorno = Entorno::nuevo();
    let temporal = pages::create(&entorno.db, "Temporal").unwrap();
    let sonido = entorno.importar("audio.wav", 8).sound().clone();
    slots::assign(&entorno.db, &temporal.id, slot(2), &sonido.id).unwrap();

    pages::delete(&entorno.db, &temporal.id).unwrap();

    assert!(sounds::find_by_id(&entorno.db, &sonido.id)
        .unwrap()
        .is_some());
    assert!(entorno.paths.sounds_dir().read_dir().unwrap().count() > 0);
}

#[test]
fn un_archivo_invalido_no_ensucia_la_biblioteca() {
    let entorno = Entorno::nuevo();
    let basura = entorno.origen.path().join("documento.mp3");
    std::fs::write(&basura, b"%PDF-1.4 esto no es audio").unwrap();

    let resultado = library::ingest(
        &entorno.db,
        &entorno.paths,
        IngestRequest::local_import(basura),
        MAX_BYTES,
    );

    assert!(resultado.is_err());
    assert_eq!(
        std::fs::read_dir(entorno.paths.sounds_dir())
            .unwrap()
            .count(),
        0
    );
    assert_eq!(sounds::facets(&entorno.db).unwrap().total, 0);
}

// --- Descarga contra un servidor HTTP local ---------------------------------

struct ServidorLocal {
    port: u16,
    server: Arc<tiny_http::Server>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ServidorLocal {
    fn iniciar(cuerpo: Vec<u8>, content_type: &'static str, status: u16) -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let hilo = server.clone();

        let handle = std::thread::spawn(move || {
            while let Ok(request) = hilo.recv() {
                let header =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes())
                        .unwrap();
                let length = cuerpo.len();
                let response = tiny_http::Response::new(
                    tiny_http::StatusCode(status),
                    vec![header],
                    Cursor::new(cuerpo.clone()),
                    Some(length),
                    None,
                );
                let _ = request.respond(response);
            }
        });

        Self {
            port,
            server,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/audio.wav", self.port)
    }
}

impl Drop for ServidorLocal {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[tokio::test]
async fn descarga_valida_termina_en_la_biblioteca() {
    let entorno = Entorno::nuevo();
    let servidor = ServidorLocal::iniciar(wav(400, 12), "audio/wav", 200);
    let temporal = entorno.paths.new_temp_file("wav");

    download_to_temp(
        &build_http_client(),
        &servidor.url(),
        &temporal,
        MAX_BYTES,
        None,
    )
    .await
    .expect("la descarga debe funcionar");

    let mut request = IngestRequest::local_import(temporal.clone());
    request.handling = SourceHandling::Move;
    request.display_name = Some("Audio remoto".into());
    request.source = SoundSource::Provider {
        provider_id: "freesound".into(),
        remote_id: "999".into(),
    };

    let resultado = library::ingest(&entorno.db, &entorno.paths, request, MAX_BYTES).unwrap();
    let sonido = resultado.sound();

    assert_eq!(sonido.name, "Audio remoto");
    assert!(sonido.file_available);
    assert!(matches!(sonido.source, SoundSource::Provider { .. }));
    assert!(!temporal.exists(), "el temporal se movio, no se copio");

    // Y queda accesible sin conexion: la ruta apunta a la carpeta administrada.
    // Canonicalizamos ambos lados con la misma funcion: en Windows conviven la
    // forma `C:\...` y la forma extendida `\\?\C:\...`.
    let ruta = library::resolve_playable_path(&entorno.db, &entorno.paths, &sonido.id).unwrap();
    let ruta = std::fs::canonicalize(&ruta).unwrap();
    let carpeta = std::fs::canonicalize(entorno.paths.sounds_dir()).unwrap();

    assert!(ruta.is_file());
    assert!(
        ruta.starts_with(&carpeta),
        "{ruta:?} deberia estar en {carpeta:?}"
    );
}

#[tokio::test]
async fn una_descarga_invalida_hace_rollback_completo() {
    let entorno = Entorno::nuevo();
    // El servidor responde 200 con un Content-Type valido pero contenido HTML.
    let servidor = ServidorLocal::iniciar(
        b"<!DOCTYPE html><html><body>error</body></html>".to_vec(),
        "application/octet-stream",
        200,
    );
    let temporal = entorno.paths.new_temp_file("wav");

    // La descarga en si funciona: el servidor devuelve bytes.
    download_to_temp(
        &build_http_client(),
        &servidor.url(),
        &temporal,
        MAX_BYTES,
        None,
    )
    .await
    .expect("bytes recibidos");

    // Pero la validacion de contenido rechaza el archivo.
    let mut request = IngestRequest::local_import(temporal.clone());
    request.handling = SourceHandling::Move;

    let error = library::ingest(&entorno.db, &entorno.paths, request, MAX_BYTES).unwrap_err();
    assert_eq!(error.kind, sound_deck_lib::errors::ErrorKind::InvalidAudio);

    // Nada quedo registrado ni copiado a la biblioteca.
    assert_eq!(sounds::facets(&entorno.db).unwrap().total, 0);
    assert_eq!(
        std::fs::read_dir(entorno.paths.sounds_dir())
            .unwrap()
            .count(),
        0
    );

    std::fs::remove_file(&temporal).ok();
}

#[tokio::test]
async fn un_error_http_no_deja_temporales() {
    let entorno = Entorno::nuevo();
    let servidor = ServidorLocal::iniciar(b"not found".to_vec(), "text/plain", 404);
    let temporal = entorno.paths.new_temp_file("wav");

    let resultado = download_to_temp(
        &build_http_client(),
        &servidor.url(),
        &temporal,
        MAX_BYTES,
        None,
    )
    .await;

    assert!(resultado.is_err());
    assert!(!temporal.exists());
    assert_eq!(entorno.paths.clean_temp().unwrap(), 0);
}

#[test]
fn la_busqueda_local_encuentra_por_nombre_y_respeta_filtros() {
    let entorno = Entorno::nuevo();
    entorno.importar("risa malvada.wav", 20);
    entorno.importar("aplauso corto.wav", 21);
    let asignado = entorno.importar("meme bruh.wav", 22).sound().clone();

    let pagina = pages::first(&entorno.db).unwrap().unwrap();
    slots::assign(&entorno.db, &pagina.id, slot(1), &asignado.id).unwrap();

    use sound_deck_lib::domain::sound::LibraryFilter;
    use sound_deck_lib::domain::SoundQuery;

    let buscar = |texto: &str, filtro: LibraryFilter| {
        sounds::search(
            &entorno.db,
            &SoundQuery {
                text: texto.to_string(),
                filter: filtro,
                ..Default::default()
            },
        )
        .unwrap()
    };

    assert_eq!(buscar("risa", LibraryFilter::All).len(), 1);
    assert_eq!(buscar("MALVADA", LibraryFilter::All).len(), 1);
    assert_eq!(buscar("", LibraryFilter::All).len(), 3);

    let sin_asignar = buscar("", LibraryFilter::Unassigned);
    assert_eq!(sin_asignar.len(), 2);
    assert!(sin_asignar.iter().all(|sonido| sonido.id != asignado.id));

    // La categoria inferida por nombre alimenta el filtro por categoria.
    let memes = buscar(
        "",
        LibraryFilter::Category {
            category: sound_deck_lib::domain::NormalizedCategory::Memes,
        },
    );
    assert_eq!(memes.len(), 1);
    assert_eq!(memes[0].id, asignado.id);
}
