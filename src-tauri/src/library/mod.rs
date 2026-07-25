//! Ingesta de audios a la biblioteca administrada.
//!
//! Es el unico camino por el que un archivo entra a la biblioteca, lo use la
//! importacion local (§10) o la descarga desde un proveedor (§14). Concentrarlo
//! aca garantiza que ambas rutas validen, deduplican y registran igual.

use std::path::{Path, PathBuf};

use crate::database::sounds::{self, NewSound};
use crate::database::Database;
use crate::domain::category::infer_category_from_filename;
use crate::domain::{NormalizedCategory, Sound, SoundLicense, SoundSource};
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::filesystem::audio_file::probe_audio_file;
use crate::filesystem::paths::{build_internal_filename, sanitize_display_name};
use crate::filesystem::{move_into_place, AppPaths};

/// Que hacer con el archivo de origen una vez copiado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceHandling {
    /// El origen es del usuario: se copia y se deja intacto.
    Copy,
    /// El origen es un temporal nuestro: se mueve.
    Move,
}

/// Peticion de ingesta de un archivo ya presente en disco.
#[derive(Debug, Clone)]
pub struct IngestRequest {
    pub source_path: PathBuf,
    pub handling: SourceHandling,
    /// Nombre visible propuesto. Si es `None` se deriva del nombre del archivo.
    pub display_name: Option<String>,
    pub source: SoundSource,
    pub source_page_url: Option<String>,
    pub download_url_reference: Option<String>,
    pub provider_category: Option<String>,
    /// Categoria sugerida por el proveedor. Si es `None` se infiere del nombre.
    pub normalized_category: Option<NormalizedCategory>,
    pub license: Option<SoundLicense>,
    pub attribution: Option<String>,
    pub tags: Vec<String>,
}

impl IngestRequest {
    /// Peticion minima para una importacion local.
    pub fn local_import(source_path: impl Into<PathBuf>) -> Self {
        Self {
            source_path: source_path.into(),
            handling: SourceHandling::Copy,
            display_name: None,
            source: SoundSource::LocalImport,
            source_page_url: None,
            download_url_reference: None,
            provider_category: None,
            normalized_category: None,
            license: None,
            attribution: None,
            tags: Vec::new(),
        }
    }
}

/// Resultado de una ingesta.
#[derive(Debug, Clone)]
pub enum IngestOutcome {
    /// Se creo un registro nuevo.
    Created(Sound),
    /// Ya existia un audio con el mismo contenido; se reutiliza (§10).
    Duplicate(Sound),
}

impl IngestOutcome {
    pub fn sound(&self) -> &Sound {
        match self {
            IngestOutcome::Created(sound) | IngestOutcome::Duplicate(sound) => sound,
        }
    }

    pub fn is_duplicate(&self) -> bool {
        matches!(self, IngestOutcome::Duplicate(_))
    }
}

/// Valida, deduplica, copia y registra un archivo de audio.
///
/// Es bloqueante (hash + decodificacion): invocala desde un hilo de blocking.
///
/// La operacion es transaccional en la practica: si el registro en la base
/// falla, el archivo recien copiado se borra, de modo que nunca queda un audio
/// huerfano en disco ni un registro apuntando a un archivo inexistente (§7).
pub fn ingest(
    db: &Database,
    paths: &AppPaths,
    request: IngestRequest,
    max_bytes: u64,
) -> AppResult<IngestOutcome> {
    let probe = probe_audio_file(&request.source_path, max_bytes)?;

    if let Some(existing) = sounds::find_by_hash(db, &probe.content_hash)? {
        tracing::info!(
            sound_id = %existing.id,
            "el audio ya estaba en la biblioteca; se reutiliza el registro existente"
        );
        if request.handling == SourceHandling::Move {
            let _ = std::fs::remove_file(&request.source_path);
        }
        return Ok(IngestOutcome::Duplicate(existing));
    }

    let original_name = request
        .source_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string);

    let display_name = request
        .display_name
        .as_deref()
        .map(sanitize_display_name)
        .unwrap_or_else(|| {
            let stem = request
                .source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Sin nombre");
            sanitize_display_name(stem)
        });

    // El nombre en disco lo controlamos nosotros: UUID + extension detectada.
    // Nada de lo que venga de una URL o de un header influye aca (§14).
    let internal_filename = build_internal_filename(&probe.extension);
    let final_path = paths.sound_file(&internal_filename);

    match request.handling {
        SourceHandling::Move => move_into_place(&request.source_path, &final_path)?,
        SourceHandling::Copy => {
            std::fs::copy(&request.source_path, &final_path).map_err(|error| {
                AppError::filesystem("No se pudo copiar el audio a la biblioteca.").with_technical(
                    format!(
                        "{} -> {}: {error}",
                        request.source_path.display(),
                        final_path.display()
                    ),
                )
            })?;
        }
    }

    let normalized_category = request.normalized_category.unwrap_or_else(|| {
        original_name
            .as_deref()
            .map(infer_category_from_filename)
            .unwrap_or(NormalizedCategory::Uncategorized)
    });

    let new_sound = NewSound {
        name: display_name,
        original_name,
        internal_filename,
        file_path: final_path.clone(),
        content_hash: probe.content_hash,
        mime_type: Some(probe.mime_type),
        file_extension: Some(probe.extension),
        file_size_bytes: i64::try_from(probe.size_bytes).ok(),
        duration_ms: probe.duration_ms,
        source: request.source,
        source_page_url: request.source_page_url,
        download_url_reference: request.download_url_reference,
        provider_category: request.provider_category,
        normalized_category,
        license: request.license,
        attribution: request.attribution,
        tags: request.tags,
    };

    match sounds::insert(db, new_sound) {
        Ok(sound) => {
            // Medir es caro pero se hace una sola vez, y ya estamos en un hilo
            // de blocking con el archivo decodificado hace un momento. Que
            // falle no invalida la importacion: el audio entra igual, sin medir.
            match crate::audio::measure_file(&final_path) {
                Ok(loudness) => {
                    if let Err(error) = sounds::save_loudness(db, &sound.id, loudness) {
                        tracing::warn!(technical = ?error.technical, "no se pudo guardar la sonoridad");
                    }
                }
                Err(error) => {
                    tracing::warn!(technical = ?error.technical, "no se pudo medir la sonoridad");
                }
            }
            Ok(IngestOutcome::Created(sound))
        }
        Err(error) => {
            // Rollback del lado del filesystem.
            if let Err(cleanup) = std::fs::remove_file(&final_path) {
                tracing::warn!(
                    path = %final_path.display(),
                    %cleanup,
                    "no se pudo revertir la copia tras fallar el registro"
                );
            }
            Err(error)
        }
    }
}

/// Resultado de importar varios archivos a la vez.
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: Vec<Sound>,
    pub duplicates: Vec<Sound>,
    pub failed: Vec<ImportFailure>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub file_name: String,
    pub message: String,
}

/// Importa una lista de archivos. Un archivo invalido no aborta el resto (§10).
pub fn import_files(
    db: &Database,
    paths: &AppPaths,
    files: &[PathBuf],
    max_bytes: u64,
) -> ImportReport {
    let mut report = ImportReport::default();

    for file in files {
        let file_name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("archivo desconocido")
            .to_string();

        match ingest(
            db,
            paths,
            IngestRequest::local_import(file.clone()),
            max_bytes,
        ) {
            Ok(IngestOutcome::Created(sound)) => report.imported.push(sound),
            Ok(IngestOutcome::Duplicate(sound)) => report.duplicates.push(sound),
            Err(error) => {
                tracing::warn!(
                    file = %file.display(),
                    technical = ?error.technical,
                    "no se pudo importar el archivo"
                );
                report.failed.push(ImportFailure {
                    file_name,
                    message: error.message.clone(),
                });
            }
        }
    }

    report
}

/// Borra un sonido: limpia sus asignaciones, el registro y el archivo.
///
/// Se hace en ese orden para no dejar registros huerfanos si el borrado del
/// archivo falla (por ejemplo, si el antivirus lo tiene tomado).
pub fn delete_sound(db: &Database, paths: &AppPaths, sound_id: &str) -> AppResult<()> {
    crate::database::slots::clear_all_uses(db, sound_id)?;
    // Se lee antes del DELETE, que se lleva la fila con la ruta de la imagen.
    let image_path = sounds::image_path_of(db, sound_id)?;
    let file_path = sounds::delete(db, sound_id)?;

    if let Some(image_path) = image_path {
        remove_managed_image(paths, &image_path);
    }

    // Solo borramos dentro de la carpeta administrada.
    match paths.assert_managed(&file_path) {
        Ok(managed) => {
            if let Err(error) = std::fs::remove_file(&managed) {
                tracing::warn!(
                    path = %managed.display(),
                    %error,
                    "el registro se borro pero el archivo quedo en disco"
                );
            }
        }
        Err(error) => {
            // Si el archivo ya no existe, no hay nada que borrar y el registro
            // ya se fue: es el resultado correcto.
            tracing::debug!(
                path = %file_path.display(),
                technical = ?error.technical,
                "no se borro el archivo: fuera de la carpeta administrada o inexistente"
            );
        }
    }

    Ok(())
}

/// Asigna una imagen a un audio: la valida, la copia a la carpeta administrada
/// y descarta la que hubiera antes.
///
/// El original del usuario queda intacto: como con los audios, la aplicacion
/// trabaja siempre sobre su propia copia, para que mover o borrar el archivo de
/// origen no rompa la botonera.
pub fn set_sound_image(
    db: &Database,
    paths: &AppPaths,
    sound_id: &str,
    source_path: &Path,
) -> AppResult<Sound> {
    if sounds::find_by_id(db, sound_id)?.is_none() {
        return Err(AppError::not_found(
            "Ese sonido ya no existe en la biblioteca.",
        ));
    }

    let probe = crate::filesystem::probe_image_file(source_path)?;

    // El nombre en disco lo elegimos nosotros: UUID + extension detectada.
    let internal_filename = build_internal_filename(&probe.extension);
    let final_path = paths.image_file(&internal_filename);

    std::fs::copy(source_path, &final_path).map_err(|error| {
        AppError::filesystem("No se pudo copiar la imagen a la carpeta de la aplicacion.")
            .with_technical(format!(
                "{} -> {}: {error}",
                source_path.display(),
                final_path.display()
            ))
    })?;

    // La anterior se borra despues de registrar la nueva: si el UPDATE falla,
    // el audio conserva la imagen que ya tenia.
    let previous = sounds::image_path_of(db, sound_id)?;

    let sound = match sounds::set_image(db, sound_id, Some(&final_path)) {
        Ok(sound) => sound,
        Err(error) => {
            let _ = std::fs::remove_file(&final_path);
            return Err(error);
        }
    };

    if let Some(previous) = previous {
        remove_managed_image(paths, &previous);
    }

    Ok(sound)
}

/// Quita la imagen de un audio y borra el archivo administrado.
pub fn clear_sound_image(db: &Database, paths: &AppPaths, sound_id: &str) -> AppResult<Sound> {
    let previous = sounds::image_path_of(db, sound_id)?;
    let sound = sounds::set_image(db, sound_id, None)?;

    if let Some(previous) = previous {
        remove_managed_image(paths, &previous);
    }

    Ok(sound)
}

/// Borra una imagen solo si esta dentro de la carpeta administrada. Que falle
/// no es un error para el usuario: el registro ya no la referencia.
fn remove_managed_image(paths: &AppPaths, path: &Path) {
    match paths.assert_managed_image(path) {
        Ok(managed) => {
            if let Err(error) = std::fs::remove_file(&managed) {
                tracing::warn!(
                    path = %managed.display(),
                    %error,
                    "no se pudo borrar la imagen anterior"
                );
            }
        }
        Err(error) => tracing::debug!(
            path = %path.display(),
            technical = ?error.technical,
            "no se borro la imagen: fuera de la carpeta administrada o inexistente"
        ),
    }
}

/// Formatos de imagen aceptados, para el filtro del dialogo nativo.
pub fn supported_image_extensions() -> &'static [&'static str] {
    &crate::filesystem::paths::SUPPORTED_IMAGE_EXTENSIONS
}

/// Elimina registros cuyo archivo ya no existe (§20 Biblioteca).
pub fn remove_orphan_records(db: &Database, paths: &AppPaths) -> AppResult<usize> {
    let missing = sounds::find_missing_files(db)?;
    let mut removed = 0;

    for sound in missing {
        match delete_sound(db, paths, &sound.id) {
            Ok(()) => removed += 1,
            Err(error) => tracing::warn!(
                sound_id = %sound.id,
                technical = ?error.technical,
                "no se pudo eliminar un registro huerfano"
            ),
        }
    }

    Ok(removed)
}

/// Ruta absoluta de un sonido, validada contra la carpeta administrada.
/// Es la unica forma de obtener una ruta para reproducir.
pub fn resolve_playable_path(
    db: &Database,
    paths: &AppPaths,
    sound_id: &str,
) -> AppResult<PathBuf> {
    let stored = sounds::file_path_of(db, sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?;

    paths.assert_managed(&stored).map_err(|error| {
        AppError::new(
            ErrorKind::Filesystem,
            "El archivo de este sonido ya no esta disponible.",
        )
        .with_detail("soundId", sound_id.to_string())
        .with_detail("reason", "missing_file")
        .with_technical(error.technical.unwrap_or_default())
    })
}

/// Copia de seguridad de la base de datos en la carpeta `backups` (§20).
pub fn backup_database(paths: &AppPaths) -> AppResult<PathBuf> {
    let source = paths.database_file();
    if !source.is_file() {
        return Err(AppError::not_found(
            "Todavia no hay una base de datos que respaldar.",
        ));
    }

    let stamp = crate::domain::now_timestamp()
        .replace([':', '-'], "")
        .replace('.', "");
    let destination = paths.backups_dir().join(format!("database-{stamp}.sqlite"));
    std::fs::copy(&source, &destination)?;
    Ok(destination)
}

/// Cuantos audios se midieron y cuantos no se pudieron medir.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessReport {
    pub measured: usize,
    pub failed: usize,
}

/// Mide los audios que entraron a la biblioteca antes de que existiera la
/// normalizacion. Es bloqueante y puede tardar: decodifica cada archivo entero.
pub fn measure_pending_loudness(db: &Database, paths: &AppPaths) -> AppResult<LoudnessReport> {
    let mut report = LoudnessReport::default();

    for (sound_id, file_path) in sounds::pending_loudness(db)? {
        // Un archivo que ya no esta no es un error de esta operacion: para eso
        // esta la limpieza de huerfanos.
        if paths.assert_managed(&file_path).is_err() || !file_path.is_file() {
            report.failed += 1;
            continue;
        }

        match crate::audio::measure_file(&file_path) {
            Ok(loudness) => match sounds::save_loudness(db, &sound_id, loudness) {
                Ok(()) => report.measured += 1,
                Err(error) => {
                    tracing::warn!(sound_id, technical = ?error.technical, "no se pudo guardar la sonoridad");
                    report.failed += 1;
                }
            },
            Err(error) => {
                tracing::warn!(sound_id, technical = ?error.technical, "no se pudo medir la sonoridad");
                report.failed += 1;
            }
        }
    }

    tracing::info!(
        medidos = report.measured,
        fallidos = report.failed,
        "medicion de sonoridad terminada"
    );
    Ok(report)
}

/// Deja una copia de seguridad lista para reemplazar a la base actual.
///
/// No se restaura en el momento: la aplicacion tiene la base abierta y en
/// Windows ni siquiera se puede sobrescribir un archivo en uso. La copia queda
/// en `database.restore.sqlite` y el reemplazo lo hace `apply_pending_restore`
/// en el proximo arranque, antes de abrir nada.
///
/// Valida antes de copiar: restaurar un archivo que no es una base nuestra
/// dejaria la aplicacion sin arrancar, y ahi ya no hay interfaz para arreglarlo.
pub fn stage_restore(paths: &AppPaths, source: &Path) -> AppResult<()> {
    let version = inspect_backup(source)?;
    tracing::info!(
        version,
        source = %source.display(),
        "copia de seguridad validada, queda pendiente de restaurar"
    );

    std::fs::copy(source, paths.database_restore_file())?;
    Ok(())
}

/// Cancela una restauracion que todavia no se aplico.
pub fn cancel_pending_restore(paths: &AppPaths) -> AppResult<bool> {
    let pending = paths.database_restore_file();
    if !pending.is_file() {
        return Ok(false);
    }
    std::fs::remove_file(&pending)?;
    Ok(true)
}

/// Version de esquema de una copia, si es una base de Sound Deck que podemos abrir.
fn inspect_backup(source: &Path) -> AppResult<i64> {
    if !source.is_file() {
        return Err(AppError::not_found("Ese archivo ya no existe."));
    }

    let invalid = |detail: &str| {
        AppError::validation("Ese archivo no es una copia de seguridad de Sound Deck.")
            .with_technical(detail.to_string())
    };

    let connection = rusqlite::Connection::open_with_flags(
        source,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| invalid(&error.to_string()))?;

    for table in ["sounds", "pages", "slots", "settings", "schema_migrations"] {
        let exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|error| invalid(&error.to_string()))?;
        if exists == 0 {
            return Err(invalid(&format!("falta la tabla {table}")));
        }
    }

    let version: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|error| invalid(&error.to_string()))?;

    // Una copia de una version mas nueva puede tener columnas que esta no
    // entiende. Migrar hacia atras no existe, asi que se rechaza de frente.
    let supported = crate::database::migrations::latest_version();
    if version > supported {
        return Err(AppError::validation(format!(
            "Esa copia es de una version mas nueva de Sound Deck (esquema {version}, esta version entiende hasta {supported}). Actualiza la aplicacion para poder restaurarla."
        )));
    }

    Ok(version)
}

/// Aplica una restauracion pendiente. Se llama al arrancar, antes de abrir la base.
///
/// Devuelve la ruta de la copia de seguridad que se guardo de la base anterior,
/// para poder informarla: reemplazar la biblioteca de alguien sin dejarle una
/// vuelta atras no es aceptable.
pub fn apply_pending_restore(paths: &AppPaths) -> AppResult<Option<PathBuf>> {
    let pending = paths.database_restore_file();
    if !pending.is_file() {
        return Ok(None);
    }

    let current = paths.database_file();
    let previous = if current.is_file() {
        Some(backup_database(paths)?)
    } else {
        None
    };

    // WAL y el indice compartido pertenecen a la base que se va: dejarlos seria
    // mezclar transacciones de una base con el contenido de otra.
    for sidecar in ["-wal", "-shm"] {
        let path = PathBuf::from(format!("{}{sidecar}", current.display()));
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
    }

    std::fs::rename(&pending, &current)?;
    tracing::info!(
        previous = ?previous.as_ref().map(|path| path.display().to_string()),
        "base de datos restaurada desde una copia de seguridad"
    );

    Ok(previous)
}

/// Extensiones aceptadas, para el filtro del dialogo nativo.
pub fn supported_extensions() -> &'static [&'static str] {
    &crate::filesystem::paths::SUPPORTED_EXTENSIONS
}

/// Ruta absoluta de un archivo, tal como la entrega el dialogo del sistema.
pub fn normalize_input_path(raw: &str) -> AppResult<PathBuf> {
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(AppError::validation(
            "Solo se aceptan rutas absolutas de archivos.",
        ));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{slots, test_db};
    use crate::domain::SlotNumber;

    fn wav(millis: u32, marca: u8) -> Vec<u8> {
        let sample_rate = 8000u32;
        let samples = sample_rate * millis / 1000;
        let data_len = samples * 2;
        let mut out = Vec::new();
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
        // La marca cambia el contenido para que el hash difiera.
        out.extend(std::iter::repeat_n(marca, data_len as usize));
        out
    }

    struct Entorno {
        db: Database,
        paths: AppPaths,
        _dir: tempfile::TempDir,
        origen: tempfile::TempDir,
    }

    fn entorno() -> Entorno {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_root(dir.path()).unwrap();
        Entorno {
            db: test_db(),
            paths,
            _dir: dir,
            origen: tempfile::tempdir().unwrap(),
        }
    }

    fn archivo(entorno: &Entorno, nombre: &str, marca: u8) -> PathBuf {
        let ruta = entorno.origen.path().join(nombre);
        std::fs::write(&ruta, wav(200, marca)).unwrap();
        ruta
    }

    const LIMITE: u64 = 10 * 1024 * 1024;

    #[test]
    fn importa_un_audio_y_lo_copia_a_la_biblioteca() {
        let e = entorno();
        let origen = archivo(&e, "risa malvada.wav", 1);

        let resultado = ingest(
            &e.db,
            &e.paths,
            IngestRequest::local_import(origen.clone()),
            LIMITE,
        )
        .unwrap();

        let sonido = resultado.sound();
        assert!(!resultado.is_duplicate());
        assert_eq!(sonido.name, "risa malvada");
        assert_eq!(sonido.original_name.as_deref(), Some("risa malvada.wav"));
        assert_eq!(sonido.file_extension.as_deref(), Some("wav"));
        assert!(sonido.duration_ms.unwrap() > 0);
        assert!(sonido.file_available);

        // El original del usuario queda intacto.
        assert!(origen.exists());
        // Y en la biblioteca hay exactamente un archivo, con nombre UUID.
        let archivos: Vec<_> = std::fs::read_dir(e.paths.sounds_dir())
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(archivos.len(), 1);
        let nombre = archivos[0].file_name().to_string_lossy().to_string();
        assert!(nombre.ends_with(".wav"));
        assert!(!nombre.contains("risa"));
    }

    #[test]
    fn detecta_duplicados_por_hash_sin_copiar_de_nuevo() {
        let e = entorno();
        let primero = archivo(&e, "a.wav", 7);
        let segundo = archivo(&e, "copia-de-a.wav", 7);

        let uno = ingest(
            &e.db,
            &e.paths,
            IngestRequest::local_import(primero),
            LIMITE,
        )
        .unwrap();
        let dos = ingest(
            &e.db,
            &e.paths,
            IngestRequest::local_import(segundo),
            LIMITE,
        )
        .unwrap();

        assert!(!uno.is_duplicate());
        assert!(dos.is_duplicate());
        assert_eq!(uno.sound().id, dos.sound().id);

        let archivos = std::fs::read_dir(e.paths.sounds_dir()).unwrap().count();
        assert_eq!(archivos, 1, "el binario no debe duplicarse");
    }

    #[test]
    fn mover_un_temporal_lo_saca_del_origen() {
        let e = entorno();
        let temporal = e.paths.new_temp_file("wav");
        std::fs::write(&temporal, wav(150, 3)).unwrap();

        let mut request = IngestRequest::local_import(temporal.clone());
        request.handling = SourceHandling::Move;
        request.display_name = Some("Desde proveedor".into());
        request.source = SoundSource::Provider {
            provider_id: "freesound".into(),
            remote_id: "42".into(),
        };

        let resultado = ingest(&e.db, &e.paths, request, LIMITE).unwrap();

        assert_eq!(resultado.sound().name, "Desde proveedor");
        assert!(!temporal.exists(), "el temporal debe haberse movido");
        assert!(matches!(
            resultado.sound().source,
            SoundSource::Provider { .. }
        ));
    }

    #[test]
    fn un_duplicado_de_temporal_borra_el_temporal() {
        let e = entorno();
        let primero = archivo(&e, "a.wav", 9);
        ingest(
            &e.db,
            &e.paths,
            IngestRequest::local_import(primero),
            LIMITE,
        )
        .unwrap();

        let temporal = e.paths.new_temp_file("wav");
        std::fs::write(&temporal, wav(200, 9)).unwrap();
        let mut request = IngestRequest::local_import(temporal.clone());
        request.handling = SourceHandling::Move;

        let resultado = ingest(&e.db, &e.paths, request, LIMITE).unwrap();
        assert!(resultado.is_duplicate());
        assert!(!temporal.exists(), "no debe quedar basura en temp");
    }

    #[test]
    fn un_archivo_invalido_no_deja_nada_en_la_biblioteca() {
        let e = entorno();
        let basura = e.origen.path().join("virus.mp3");
        std::fs::write(&basura, b"MZ\x90\x00 esto es un ejecutable").unwrap();

        let error =
            ingest(&e.db, &e.paths, IngestRequest::local_import(basura), LIMITE).unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidAudio);
        assert_eq!(std::fs::read_dir(e.paths.sounds_dir()).unwrap().count(), 0);
    }

    #[test]
    fn importar_varios_separa_exitos_duplicados_y_fallos() {
        let e = entorno();
        let uno = archivo(&e, "uno.wav", 1);
        let dos = archivo(&e, "dos.wav", 2);
        let repetido = archivo(&e, "repetido.wav", 1);
        let roto = e.origen.path().join("roto.wav");
        std::fs::write(&roto, b"no soy audio").unwrap();

        let reporte = import_files(&e.db, &e.paths, &[uno, dos, repetido, roto], LIMITE);

        assert_eq!(reporte.imported.len(), 2);
        assert_eq!(reporte.duplicates.len(), 1);
        assert_eq!(reporte.failed.len(), 1);
        assert_eq!(reporte.failed[0].file_name, "roto.wav");
        assert!(!reporte.failed[0].message.is_empty());
    }

    #[test]
    fn infiere_categoria_por_nombre_solo_cuando_es_evidente() {
        let e = entorno();
        let meme = archivo(&e, "meme-risa.wav", 11);
        let neutro = archivo(&e, "grabacion-001.wav", 12);

        let a = ingest(&e.db, &e.paths, IngestRequest::local_import(meme), LIMITE).unwrap();
        let b = ingest(&e.db, &e.paths, IngestRequest::local_import(neutro), LIMITE).unwrap();

        assert_eq!(a.sound().normalized_category, NormalizedCategory::Memes);
        assert_eq!(
            b.sound().normalized_category,
            NormalizedCategory::Uncategorized
        );
    }

    #[test]
    fn borrar_un_sonido_limpia_slots_registro_y_archivo() {
        let e = entorno();
        let pagina = crate::database::pages::create(&e.db, "Principal").unwrap();
        let origen = archivo(&e, "a.wav", 5);
        let sonido = ingest(&e.db, &e.paths, IngestRequest::local_import(origen), LIMITE)
            .unwrap()
            .sound()
            .clone();
        slots::assign(&e.db, &pagina.id, SlotNumber::new(1).unwrap(), &sonido.id).unwrap();

        delete_sound(&e.db, &e.paths, &sonido.id).unwrap();

        assert!(sounds::find_by_id(&e.db, &sonido.id).unwrap().is_none());
        assert!(slots::get(&e.db, &pagina.id, SlotNumber::new(1).unwrap())
            .unwrap()
            .unwrap()
            .sound
            .is_none());
        assert_eq!(std::fs::read_dir(e.paths.sounds_dir()).unwrap().count(), 0);
    }

    #[test]
    fn resolver_la_ruta_falla_si_el_archivo_desaparecio() {
        let e = entorno();
        let origen = archivo(&e, "a.wav", 4);
        let sonido = ingest(&e.db, &e.paths, IngestRequest::local_import(origen), LIMITE)
            .unwrap()
            .sound()
            .clone();

        assert!(resolve_playable_path(&e.db, &e.paths, &sonido.id).is_ok());

        // Alguien borro el archivo por fuera de la aplicacion (§39).
        for entry in std::fs::read_dir(e.paths.sounds_dir()).unwrap().flatten() {
            std::fs::remove_file(entry.path()).unwrap();
        }

        let error = resolve_playable_path(&e.db, &e.paths, &sonido.id).unwrap_err();
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("missing_file")
        );
    }

    #[test]
    fn limpia_registros_huerfanos() {
        let e = entorno();
        let origen = archivo(&e, "a.wav", 6);
        ingest(&e.db, &e.paths, IngestRequest::local_import(origen), LIMITE).unwrap();

        for entry in std::fs::read_dir(e.paths.sounds_dir()).unwrap().flatten() {
            std::fs::remove_file(entry.path()).unwrap();
        }

        assert_eq!(sounds::find_missing_files(&e.db).unwrap().len(), 1);
        assert_eq!(remove_orphan_records(&e.db, &e.paths).unwrap(), 1);
        assert_eq!(sounds::find_missing_files(&e.db).unwrap().len(), 0);
    }

    #[test]
    fn rechaza_rutas_relativas() {
        assert!(normalize_input_path("relativa/audio.mp3").is_err());
        assert!(normalize_input_path("../../etc/passwd").is_err());
    }

    /// Crea una base real en disco, que es lo unico que sirve para probar la
    /// restauracion: el archivo tiene que existir y tener nuestro esquema.
    fn base_en_disco(ruta: &Path, nombre_pagina: &str) {
        let db = Database::open(ruta).unwrap();
        db.ensure_initial_state().unwrap();
        let pagina = crate::database::pages::first(&db).unwrap().unwrap();
        crate::database::pages::rename(&db, &pagina.id, nombre_pagina).unwrap();
    }

    #[test]
    fn restaurar_reemplaza_la_base_y_deja_la_anterior_a_salvo() {
        let e = entorno();
        base_en_disco(&e.paths.database_file(), "La de antes");

        let copia = e.origen.path().join("database-vieja.sqlite");
        base_en_disco(&copia, "La restaurada");

        stage_restore(&e.paths, &copia).unwrap();
        // Hasta que no se arranque de nuevo, la base activa sigue siendo la de antes.
        assert!(e.paths.database_restore_file().is_file());

        let anterior = apply_pending_restore(&e.paths).unwrap().unwrap();

        let db = Database::open(&e.paths.database_file()).unwrap();
        let pagina = crate::database::pages::first(&db).unwrap().unwrap();
        assert_eq!(pagina.name, "La restaurada");

        // La biblioteca anterior no se pierde: queda una copia con su nombre.
        assert!(anterior.is_file());
        let vieja = Database::open(&anterior).unwrap();
        assert_eq!(
            crate::database::pages::first(&vieja).unwrap().unwrap().name,
            "La de antes"
        );

        // Y la pendiente se consumio: no se vuelve a aplicar en cada arranque.
        assert!(!e.paths.database_restore_file().is_file());
        assert!(apply_pending_restore(&e.paths).unwrap().is_none());
    }

    #[test]
    fn no_acepta_como_copia_algo_que_no_es_una_base_nuestra() {
        let e = entorno();

        let cualquiera = e.origen.path().join("foto.sqlite");
        std::fs::write(&cualquiera, b"esto no es una base de datos").unwrap();
        assert!(stage_restore(&e.paths, &cualquiera).is_err());

        // Una base de SQLite valida pero de otra aplicacion tampoco sirve.
        let ajena = e.origen.path().join("ajena.sqlite");
        let connection = rusqlite::Connection::open(&ajena).unwrap();
        connection
            .execute_batch("CREATE TABLE cosas (id TEXT);")
            .unwrap();
        drop(connection);
        assert!(stage_restore(&e.paths, &ajena).is_err());

        assert!(stage_restore(&e.paths, &e.origen.path().join("no-existe.sqlite")).is_err());

        // Ningun rechazo deja una restauracion a medias esperando el arranque.
        assert!(!e.paths.database_restore_file().is_file());
    }

    #[test]
    fn rechaza_una_copia_de_una_version_mas_nueva() {
        let e = entorno();
        let futura = e.origen.path().join("futura.sqlite");
        base_en_disco(&futura, "Del futuro");

        let connection = rusqlite::Connection::open(&futura).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, 'del_futuro', 'now')",
                [crate::database::migrations::latest_version() + 1],
            )
            .unwrap();
        drop(connection);

        let error = stage_restore(&e.paths, &futura).unwrap_err();
        assert!(error.message.contains("mas nueva"), "{}", error.message);
        assert!(!e.paths.database_restore_file().is_file());
    }

    #[test]
    fn importar_deja_el_audio_medido_y_listo_para_normalizar() {
        let e = entorno();
        let origen = archivo(&e, "medido.wav", 7);

        let resultado =
            ingest(&e.db, &e.paths, IngestRequest::local_import(origen), LIMITE).unwrap();
        let sonido = resultado.sound();

        let medida = sounds::loudness_of(&e.db, &sonido.id).unwrap().unwrap();
        assert!(
            medida.peak > 0.0,
            "un audio con contenido no puede tener pico 0"
        );
        assert!(medida.lufs.is_finite());

        // Y no queda nada pendiente de medir.
        assert!(sounds::pending_loudness(&e.db).unwrap().is_empty());
    }

    #[test]
    fn medir_la_biblioteca_alcanza_a_los_audios_que_entraron_sin_medicion() {
        let e = entorno();
        let origen = archivo(&e, "viejo.wav", 8);
        let sonido = ingest(&e.db, &e.paths, IngestRequest::local_import(origen), LIMITE)
            .unwrap()
            .sound()
            .clone();

        // Simulamos un audio de antes de que existiera la normalizacion.
        {
            let connection = e.db.lock();
            connection
                .execute(
                    "UPDATE sounds SET loudness_lufs = NULL, peak_amplitude = NULL WHERE id = ?1",
                    [&sonido.id],
                )
                .unwrap();
        }
        assert_eq!(sounds::pending_loudness(&e.db).unwrap().len(), 1);

        let reporte = measure_pending_loudness(&e.db, &e.paths).unwrap();
        assert_eq!(reporte.measured, 1);
        assert_eq!(reporte.failed, 0);
        assert!(sounds::loudness_of(&e.db, &sonido.id).unwrap().is_some());

        // Correrlo de nuevo no vuelve a medir lo ya medido.
        let repetido = measure_pending_loudness(&e.db, &e.paths).unwrap();
        assert_eq!(repetido.measured, 0);
    }

    #[test]
    fn se_puede_cancelar_una_restauracion_pendiente() {
        let e = entorno();
        let copia = e.origen.path().join("copia.sqlite");
        base_en_disco(&copia, "Otra");

        stage_restore(&e.paths, &copia).unwrap();
        assert!(cancel_pending_restore(&e.paths).unwrap());
        assert!(!cancel_pending_restore(&e.paths).unwrap());
        assert!(apply_pending_restore(&e.paths).unwrap().is_none());
    }
}
