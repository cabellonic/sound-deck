//! Comandos de dispositivos de salida (§18).

use tauri::State;

use crate::audio::{list_output_devices, AudioDeviceInfo};
use crate::database::settings as settings_repo;
use crate::domain::settings::effective_volume;
use crate::errors::AppResult;
use crate::state::AppState;

#[tauri::command(async)]
pub fn list_audio_devices(state: State<'_, AppState>) -> AppResult<AudioDeviceList> {
    let devices = list_output_devices()?;
    Ok(AudioDeviceList {
        current: state.audio.current_device(),
        devices,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceList {
    pub devices: Vec<AudioDeviceInfo>,
    /// Dispositivo abierto ahora mismo, que puede no ser el guardado si hubo fallback.
    pub current: Option<AudioDeviceInfo>,
}

/// Selecciona un dispositivo de salida y persiste la eleccion.
///
/// Solo se guarda si la apertura funciono: no queremos persistir una
/// preferencia que ya sabemos que falla (§17.8 aplicado al audio).
#[tauri::command(async)]
pub fn select_audio_device(
    state: State<'_, AppState>,
    device_id: Option<String>,
    device_name: Option<String>,
) -> AppResult<AudioDeviceInfo> {
    let (info, _resolution) = state
        .audio
        .open_device(device_id.as_deref(), device_name.as_deref())?;

    settings_repo::save_output_device(&state.db, info.id.clone(), Some(info.name.clone()))?;
    Ok(info)
}

/// Vuelve al dispositivo predeterminado del sistema.
#[tauri::command(async)]
pub fn use_default_audio_device(state: State<'_, AppState>) -> AppResult<AudioDeviceInfo> {
    let (info, _) = state.audio.open_device(None, None)?;
    settings_repo::save_output_device(&state.db, None, None)?;
    Ok(info)
}

/// Reproduce un tono corto generado en memoria sobre el dispositivo actual.
#[tauri::command(async)]
pub fn test_audio_device(state: State<'_, AppState>) -> AppResult<()> {
    let settings = state.settings()?;
    state
        .audio
        .play_test_tone(effective_volume(settings.audio.master_volume, None, None))
}
