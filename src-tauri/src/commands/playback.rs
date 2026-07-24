//! Comandos de reproduccion y previsualizacion (§18, §27).

use tauri::{AppHandle, Manager, State};

use crate::database::{slots, sounds};
use crate::domain::settings::{effective_preview_volume, effective_volume};
use crate::domain::SlotNumber;
use crate::errors::{AppError, AppResult};
use crate::library;
use crate::state::AppState;

/// Reproduce un sonido de la biblioteca.
#[tauri::command(async)]
pub fn play_sound(state: State<'_, AppState>, sound_id: String) -> AppResult<()> {
    let settings = state.settings()?;
    let sound = sounds::find_by_id(&state.db, &sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?;

    let path = library::resolve_playable_path(&state.db, &state.paths, &sound_id)?;
    let volume = effective_volume(settings.audio.master_volume, sound.custom_volume, None);

    state.audio.play_file(
        &path,
        volume,
        settings.audio.playback_mode,
        Some(sound_id.clone()),
        settings.audio.restart_same_sound,
    )?;

    let _ = sounds::record_play(&state.db, &sound_id);
    Ok(())
}

/// Reproduce el sonido asignado a un slot.
///
/// El volumen del boton gana sobre el del audio, y el del audio sobre el general.
#[tauri::command(async)]
pub fn play_slot(
    state: State<'_, AppState>,
    page_id: String,
    slot_number: SlotNumber,
) -> AppResult<()> {
    let slot = slots::get(&state.db, &page_id, slot_number)?
        .ok_or_else(|| AppError::not_found("Ese slot no existe."))?;

    let Some(sound) = slot.sound else {
        // Un slot vacio no es un error: simplemente no suena nada (§39).
        return Ok(());
    };

    let settings = state.settings()?;
    let path = library::resolve_playable_path(&state.db, &state.paths, &sound.id)?;
    let volume = effective_volume(
        settings.audio.master_volume,
        sound.custom_volume,
        slot.custom_volume,
    );

    state.audio.play_file(
        &path,
        volume,
        settings.audio.playback_mode,
        Some(sound.id.clone()),
        settings.audio.restart_same_sound,
    )?;

    let _ = sounds::record_play(&state.db, &sound.id);
    Ok(())
}

/// Previsualiza un sonido local con el volumen de preview.
#[tauri::command(async)]
pub fn preview_local_sound(state: State<'_, AppState>, sound_id: String) -> AppResult<()> {
    let settings = state.settings()?;
    let sound = sounds::find_by_id(&state.db, &sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?;

    let path = library::resolve_playable_path(&state.db, &state.paths, &sound_id)?;
    let volume = effective_preview_volume(settings.audio.preview_volume, sound.custom_volume);

    state.audio.preview_file(&path, volume, None)
}

/// Previsualiza un resultado online sin guardarlo en la biblioteca (§27).
///
/// El audio se baja a un temporal que se borra al detener la preview: no queda
/// nada permanente hasta que el usuario decide guardarlo.
#[tauri::command]
pub async fn preview_remote_sound(
    app: AppHandle,
    provider_id: String,
    remote_id: String,
) -> AppResult<()> {
    let (client, temp_path, preview_url, max_bytes, volume) = {
        let state = app.state::<AppState>();
        let settings = state.settings()?;

        let provider = state
            .providers
            .get(&provider_id)
            .ok_or_else(|| AppError::not_found("Ese proveedor no esta disponible."))?;
        if !provider.capabilities().preview {
            return Err(AppError::validation(
                "Este proveedor no permite previsualizar.",
            ));
        }

        let remote = state
            .remote_sound(&provider_id, &remote_id)
            .ok_or_else(|| {
                AppError::not_found("Ese resultado ya no esta disponible. Repeti la busqueda.")
            })?;
        let preview_url = remote.preview_url.clone().ok_or_else(|| {
            AppError::validation("Este resultado no ofrece una previsualizacion.")
        })?;

        // La URL de preview se valida igual que la de descarga.
        crate::downloads::url::validate_remote_url(&preview_url, provider.allowed_hosts())?;

        (
            state.http.clone(),
            state.paths.new_temp_file("mp3"),
            preview_url,
            settings.audio.max_download_bytes,
            // Un resultado online todavia no esta en la biblioteca: no tiene
            // volumen propio, asi que manda el de previsualizacion.
            effective_preview_volume(settings.audio.preview_volume, None),
        )
    };

    crate::downloads::download_to_temp(&client, &preview_url, &temp_path, max_bytes, None).await?;

    let state = app.state::<AppState>();
    state
        .audio
        .preview_file(&temp_path, volume, Some(temp_path.clone()))
}

#[tauri::command(async)]
pub fn stop_preview(state: State<'_, AppState>) {
    state.audio.stop_preview();
}

#[tauri::command(async)]
pub fn stop_all(state: State<'_, AppState>) {
    state.audio.stop_all();
}

/// Estado de reproduccion para que la interfaz muestre los indicadores.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub playing_sound_ids: Vec<String>,
    pub previewing: bool,
}

#[tauri::command(async)]
pub fn get_playback_status(state: State<'_, AppState>) -> PlaybackStatus {
    PlaybackStatus {
        playing_sound_ids: state.audio.playing_sound_ids(),
        previewing: state.audio.is_previewing(),
    }
}
