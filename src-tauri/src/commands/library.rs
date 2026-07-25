//! Comandos de la biblioteca local: busqueda, importacion y mantenimiento.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::database::sounds::{self, LibraryFacets};
use crate::domain::SoundUsage;
use crate::domain::{Sound, SoundQuery};
use crate::errors::{AppError, AppResult};
use crate::events;
use crate::filesystem::audio_file::format_bytes;
use crate::library::{self, ImportReport};
use crate::state::AppState;

#[tauri::command(async)]
pub fn search_local_sounds(
    state: State<'_, AppState>,
    query: Option<SoundQuery>,
) -> AppResult<Vec<Sound>> {
    sounds::search(&state.db, &query.unwrap_or_default())
}

/// Contadores para los filtros automaticos. Solo aparecen los que existen (§9).
#[tauri::command(async)]
pub fn get_library_facets(state: State<'_, AppState>) -> AppResult<LibraryFacets> {
    sounds::facets(&state.db)
}

/// Importa archivos elegidos por el usuario.
///
/// El trabajo pesado (hash, decodificacion, copia) va a un hilo de blocking:
/// nunca bloqueamos el hilo principal de Tauri (§4.6).
#[tauri::command]
pub async fn import_sound_files(app: AppHandle, paths: Vec<String>) -> AppResult<ImportReport> {
    if paths.is_empty() {
        return Ok(ImportReport::default());
    }
    if paths.len() > 200 {
        return Err(AppError::validation(
            "Se pueden importar hasta 200 archivos por vez.",
        ));
    }

    let files: Vec<PathBuf> = paths
        .iter()
        .map(|path| library::normalize_input_path(path))
        .collect::<AppResult<Vec<_>>>()?;

    let report = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            let state = app.state::<AppState>();
            let max_bytes = state
                .settings()
                .map(|settings| settings.audio.max_download_bytes)
                .unwrap_or(25 * 1024 * 1024);
            library::import_files(&state.db, &state.paths, &files, max_bytes)
        }
    })
    .await
    .map_err(|error| {
        AppError::filesystem("La importacion se interrumpio.").with_technical(error.to_string())
    })?;

    events::emit(&app, events::LIBRARY_CHANGED, ());

    tracing::info!(
        importados = report.imported.len(),
        duplicados = report.duplicates.len(),
        fallidos = report.failed.len(),
        "importacion finalizada"
    );
    Ok(report)
}

#[tauri::command(async)]
pub fn rename_sound(
    app: AppHandle,
    state: State<'_, AppState>,
    sound_id: String,
    name: String,
) -> AppResult<Sound> {
    let sound = sounds::rename(&state.db, &sound_id, &name)?;
    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(sound)
}

/// Volumen absoluto propio del audio. `null` lo vuelve a linkear al general.
#[tauri::command(async)]
pub fn update_sound_volume(
    app: AppHandle,
    state: State<'_, AppState>,
    sound_id: String,
    volume: Option<f32>,
) -> AppResult<Sound> {
    let sound = sounds::update_volume(&state.db, &sound_id, volume)?;
    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(sound)
}

/// Asigna la imagen que la botonera muestra para este audio.
///
/// La validacion y la copia son bloqueantes, asi que van a un hilo aparte.
#[tauri::command]
pub async fn set_sound_image(app: AppHandle, sound_id: String, path: String) -> AppResult<Sound> {
    let source = library::normalize_input_path(&path)?;

    let sound = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            let state = app.state::<AppState>();
            library::set_sound_image(&state.db, &state.paths, &sound_id, &source)
        }
    })
    .await
    .map_err(|error| {
        AppError::filesystem("La asignacion de la imagen se interrumpio.")
            .with_technical(error.to_string())
    })??;

    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(sound)
}

#[tauri::command(async)]
pub fn clear_sound_image(
    app: AppHandle,
    state: State<'_, AppState>,
    sound_id: String,
) -> AppResult<Sound> {
    let sound = library::clear_sound_image(&state.db, &state.paths, &sound_id)?;
    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(sound)
}

/// Extensiones de imagen aceptadas, para el dialogo nativo.
#[tauri::command]
pub fn supported_image_extensions() -> Vec<String> {
    library::supported_image_extensions()
        .iter()
        .map(|extension| extension.to_string())
        .collect()
}

#[tauri::command(async)]
pub fn update_sound_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    sound_id: String,
    tags: Vec<String>,
) -> AppResult<Sound> {
    let sound = sounds::set_tags(&state.db, &sound_id, &tags)?;
    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(sound)
}

/// Donde esta usado un sonido. El frontend lo muestra antes de confirmar
/// el borrado, para que el usuario sepa que va a perder (§9).
#[tauri::command(async)]
pub fn get_sound_usage(state: State<'_, AppState>, sound_id: String) -> AppResult<Vec<SoundUsage>> {
    sounds::usage(&state.db, &sound_id)
}

#[tauri::command(async)]
pub fn delete_sound(app: AppHandle, state: State<'_, AppState>, sound_id: String) -> AppResult<()> {
    library::delete_sound(&state.db, &state.paths, &sound_id)?;
    events::emit(&app, events::LIBRARY_CHANGED, ());
    Ok(())
}

/// Estadisticas de la carpeta de sonidos para la seccion Biblioteca de Ajustes.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStorage {
    pub sounds_dir: String,
    pub used_bytes: u64,
    pub used_readable: String,
    pub missing_files: usize,
}

#[tauri::command(async)]
pub fn get_library_storage(state: State<'_, AppState>) -> AppResult<LibraryStorage> {
    let used_bytes = state.paths.sounds_size_bytes();
    Ok(LibraryStorage {
        sounds_dir: state.paths.sounds_dir().to_string_lossy().to_string(),
        used_readable: format_bytes(used_bytes),
        used_bytes,
        missing_files: sounds::find_missing_files(&state.db)?.len(),
    })
}

#[tauri::command(async)]
pub fn find_missing_sounds(state: State<'_, AppState>) -> AppResult<Vec<Sound>> {
    sounds::find_missing_files(&state.db)
}

#[tauri::command(async)]
pub fn remove_orphan_sounds(app: AppHandle, state: State<'_, AppState>) -> AppResult<usize> {
    let removed = library::remove_orphan_records(&state.db, &state.paths)?;
    if removed > 0 {
        events::emit(&app, events::LIBRARY_CHANGED, ());
    }
    Ok(removed)
}

#[tauri::command(async)]
pub fn clean_temp_files(state: State<'_, AppState>) -> AppResult<u64> {
    state.paths.clean_temp()
}

/// Mide la sonoridad de los audios que todavia no la tienen.
///
/// Decodifica archivo por archivo, asi que va a un hilo de blocking.
#[tauri::command]
pub async fn measure_library_loudness(app: AppHandle) -> AppResult<library::LoudnessReport> {
    let report = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            let state = app.state::<AppState>();
            library::measure_pending_loudness(&state.db, &state.paths)
        }
    })
    .await
    .map_err(|error| {
        AppError::filesystem("La medicion de volumen se interrumpio.")
            .with_technical(error.to_string())
    })??;

    if report.measured > 0 {
        events::emit(&app, events::LIBRARY_CHANGED, ());
    }
    Ok(report)
}

#[tauri::command(async)]
pub fn backup_database(state: State<'_, AppState>) -> AppResult<String> {
    let path = library::backup_database(&state.paths)?;
    Ok(path.to_string_lossy().to_string())
}

/// Restaura una copia de seguridad y reinicia la aplicacion.
///
/// La copia se valida antes de tocar nada. El reemplazo real ocurre en el
/// arranque siguiente, que es el unico momento en el que la base no esta
/// abierta; por eso esto termina reiniciando y no devuelve nunca.
#[tauri::command(async)]
pub fn restore_database(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let source = library::normalize_input_path(&path)?;
    library::stage_restore(&state.paths, &source)?;

    // El audio suena hasta que el proceso muere; pararlo antes evita dejar el
    // dispositivo tomado en el reinicio.
    state.audio.stop_all();
    app.restart();
}

/// Extensiones aceptadas, para configurar el filtro del dialogo nativo.
#[tauri::command]
pub fn supported_audio_extensions() -> Vec<String> {
    library::supported_extensions()
        .iter()
        .map(|extension| extension.to_string())
        .collect()
}

/// Carpetas que el usuario puede abrir desde la interfaz. Devolvemos la ruta
/// para que el frontend la pase al plugin `opener`; no ejecutamos comandos
/// arbitrarios del shell (§30).
#[tauri::command(async)]
pub fn get_app_folders(state: State<'_, AppState>) -> AppResult<AppFolders> {
    Ok(AppFolders {
        sounds: state.paths.sounds_dir().to_string_lossy().to_string(),
        logs: state.paths.logs_dir().to_string_lossy().to_string(),
        data: state.paths.root().to_string_lossy().to_string(),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppFolders {
    pub sounds: String,
    pub logs: String,
    pub data: String,
}

/// Ruta absoluta de un sonido, validada, para "abrir ubicacion del archivo".
#[tauri::command(async)]
pub fn reveal_sound_in_folder(state: State<'_, AppState>, sound_id: String) -> AppResult<String> {
    let path = library::resolve_playable_path(&state.db, &state.paths, &sound_id)?;
    Ok(path.to_string_lossy().to_string())
}
