//! Eventos que Rust emite hacia el frontend (§24).
//!
//! Todo estado que se origina en el backend viaja por aqui; el frontend no
//! hace polling. Los eventos de alta frecuencia (progreso de descarga) se
//! limitan en su origen, no en la interfaz.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const PLAYBACK_STARTED: &str = "playback-started";
pub const PLAYBACK_STOPPED: &str = "playback-stopped";
pub const PLAYBACK_ERROR: &str = "playback-error";
pub const DOWNLOAD_PROGRESS: &str = "download-progress";
pub const DOWNLOAD_COMPLETED: &str = "download-completed";
pub const DOWNLOAD_FAILED: &str = "download-failed";
pub const AUDIO_DEVICE_CHANGED: &str = "audio-device-changed";
pub const AUDIO_DEVICE_LOST: &str = "audio-device-lost";
pub const PAGE_CHANGED: &str = "page-changed";
pub const SLOT_CHANGED: &str = "slot-changed";
pub const LIBRARY_CHANGED: &str = "library-changed";
pub const SHORTCUT_TRIGGERED: &str = "shortcut-triggered";
pub const OVERLAY_VISIBILITY_CHANGED: &str = "overlay-visibility-changed";
pub const OVERLAY_PLACEMENT_CHANGED: &str = "overlay-placement-changed";
pub const SETTINGS_CHANGED: &str = "settings-changed";
pub const NOTICE: &str = "notice";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPayload {
    /// `None` cuando se trata de una previsualizacion o de una prueba de dispositivo.
    pub sound_id: Option<String>,
    pub is_preview: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackErrorPayload {
    pub sound_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressPayload {
    /// Identificador de la operacion, elegido por quien la inicia.
    pub operation_id: String,
    pub provider_id: String,
    pub remote_id: String,
    pub received_bytes: u64,
    /// `None` si el servidor no envia `Content-Length` (§39).
    pub total_bytes: Option<u64>,
}

/// Los eventos de fin de descarga repiten `provider_id` y `remote_id` para que
/// el frontend pueda limpiar el estado del resultado exacto que estaba bajando.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCompletedPayload {
    pub operation_id: String,
    pub provider_id: String,
    pub remote_id: String,
    pub sound_id: String,
    /// `true` si el audio ya estaba en la biblioteca y se reutilizo.
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFailedPayload {
    pub operation_id: String,
    pub provider_id: String,
    pub remote_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChangedPayload {
    pub device_name: String,
    pub device_id: Option<String>,
    /// Mensaje discreto cuando no se pudo respetar la eleccion guardada.
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotChangedPayload {
    pub page_id: String,
    pub slot_number: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutTriggeredPayload {
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayVisibilityPayload {
    pub visible: bool,
}

/// Severidad de un aviso mostrado como toast (§33).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticePayload {
    pub level: NoticeLevel,
    pub message: String,
}

/// Emite un evento a todas las ventanas. Un fallo al emitir se registra pero
/// nunca interrumpe la operacion en curso.
pub fn emit<T: Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    if let Err(error) = app.emit(event, payload) {
        tracing::warn!(event, %error, "no se pudo emitir el evento");
    }
}

/// Atajo para mostrar un aviso en la interfaz.
pub fn notify(app: &AppHandle, level: NoticeLevel, message: impl Into<String>) {
    emit(
        app,
        NOTICE,
        NoticePayload {
            level,
            message: message.into(),
        },
    );
}
