//! Comandos de ventanas, overlay y estado general de la aplicacion.

use tauri::{AppHandle, State};

use crate::database::pages;
use crate::domain::settings::AppSettings;
use crate::domain::{PageSummary, SoundPage};
use crate::errors::AppResult;
use crate::overlay;
use crate::providers::registry::ProviderStatus;
use crate::state::AppState;

#[tauri::command(async)]
pub fn show_overlay(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    overlay::show(&app, &state.overlay)
}

#[tauri::command(async)]
pub fn hide_overlay(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    overlay::hide(&app, &state.overlay)
}

#[tauri::command(async)]
pub fn toggle_overlay(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    overlay::toggle(&app, &state.overlay)
}

#[tauri::command(async)]
pub fn focus_main_window(app: AppHandle) -> AppResult<()> {
    overlay::focus_main_window(&app)
}

/// Abre el overlay para arrastrarlo a la posicion que el usuario quiera (§16).
#[tauri::command(async)]
pub fn begin_overlay_placement(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    overlay::begin_placement(&app, &state.overlay)
}

#[tauri::command(async)]
pub fn save_overlay_placement(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::settings::OverlayPosition> {
    overlay::save_placement(&app, &state.overlay)
}

#[tauri::command(async)]
pub fn cancel_overlay_placement(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    overlay::cancel_placement(&app, &state.overlay)
}

/// Vuelve al centrado automatico, olvidando la posicion elegida.
#[tauri::command(async)]
pub fn clear_overlay_placement(app: AppHandle) -> AppResult<()> {
    overlay::clear_placement(&app)
}

/// Estado inicial completo. La interfaz lo pide una sola vez al arrancar en
/// lugar de encadenar cinco llamadas.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateSnapshot {
    pub settings: AppSettings,
    pub pages: Vec<PageSummary>,
    pub active_page: Option<SoundPage>,
    pub providers: Vec<ProviderStatus>,
    pub audio_device: Option<crate::audio::AudioDeviceInfo>,
    pub version: String,
    /// Si esta plataforma puede devolver el foco a la aplicacion anterior (§16).
    pub supports_focus_restore: bool,
    pub overlay_visible: bool,
}

#[tauri::command(async)]
pub fn get_app_state(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppStateSnapshot> {
    let settings = state.settings()?;

    let active_page_id = state
        .active_page()
        .or_else(|| settings.general.last_page_id.clone());

    let active_page = match active_page_id {
        Some(id) => match pages::get(&state.db, &id)? {
            Some(page) => Some(page),
            None => pages::first(&state.db)?,
        },
        None => pages::first(&state.db)?,
    };

    if let Some(page) = &active_page {
        state.set_active_page(Some(page.id.clone()));
    }

    Ok(AppStateSnapshot {
        pages: pages::list_summaries(&state.db)?,
        providers: state.providers.statuses(&state.db)?,
        audio_device: state.audio.current_device(),
        version: app.package_info().version.to_string(),
        supports_focus_restore: crate::platform::supports_focus_restore(),
        overlay_visible: overlay::is_visible(&app),
        settings,
        active_page,
    })
}

/// Marca el onboarding como completado (§32: debe poder omitirse).
#[tauri::command(async)]
pub fn complete_onboarding(state: State<'_, AppState>) -> AppResult<()> {
    let mut settings = state.settings()?;
    settings.general.onboarding_completed = true;
    crate::database::settings::save(&state.db, &settings)
}
