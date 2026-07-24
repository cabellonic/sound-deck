//! Comandos de configuracion y atajos (§17, §20).

use tauri::{AppHandle, State};

use crate::database::settings as settings_repo;
use crate::domain::settings::{
    AppSettings, SettingsPatch, ShortcutAction, ShortcutBinding, ShortcutScope, ShortcutSettings,
};
use crate::errors::{AppError, AppResult};
use crate::events;
use crate::shortcuts::{self, RegistrationReport, ShortcutConflict};
use crate::state::AppState;

#[tauri::command(async)]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.settings()
}

#[tauri::command(async)]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> AppResult<AppSettings> {
    let settings = settings_repo::apply_patch(&state.db, patch)?;
    apply_log_level(&app, &settings);
    events::emit(&app, events::SETTINGS_CHANGED, settings.clone());
    Ok(settings)
}

/// El nivel de logs se aplica en caliente, sin reiniciar la aplicacion.
fn apply_log_level(app: &AppHandle, settings: &AppSettings) {
    use tauri::Manager;

    if let Some(handle) = app.try_state::<crate::logging::LogHandle>() {
        handle.set_level(&settings.library.log_level);
    }
}

#[tauri::command(async)]
pub fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppSettings> {
    let settings = settings_repo::reset(&state.db)?;
    apply_log_level(&app, &settings);
    shortcuts::apply(&app, &state.shortcuts, &settings.shortcuts);
    events::emit(&app, events::SETTINGS_CHANGED, settings.clone());
    Ok(settings)
}

/// Resultado de intentar cambiar un atajo.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutUpdate {
    pub settings: ShortcutSettings,
    pub conflicts: Vec<ShortcutConflict>,
    pub registration: RegistrationReport,
    /// Acelerador finalmente aplicado, ya normalizado.
    pub applied: String,
}

/// Cambia el atajo de una accion siguiendo el flujo de §17:
/// normalizar, validar, detectar conflictos, registrar y recien ahi persistir.
#[tauri::command(async)]
pub fn register_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    action: ShortcutAction,
    accelerator: String,
) -> AppResult<ShortcutUpdate> {
    let mut settings = state.settings()?;
    let previous = settings.shortcuts.clone();

    let scope = previous
        .bindings
        .iter()
        .find(|binding| binding.action == action)
        .map(|binding| binding.scope)
        .unwrap_or(ShortcutScope::Global);

    let normalized = shortcuts::validate_accelerator(&accelerator, scope)?;

    // Aplicamos el cambio sobre una copia para poder revertirlo sin tocar la base.
    let mut candidate = previous.clone();
    match candidate
        .bindings
        .iter_mut()
        .find(|binding| binding.action == action)
    {
        Some(binding) => binding.accelerator = normalized.clone(),
        None => candidate.bindings.push(ShortcutBinding {
            action,
            accelerator: normalized.clone(),
            scope,
        }),
    }

    let conflicts = shortcuts::detect_conflicts(&candidate.bindings);
    if !conflicts.is_empty() {
        return Err(AppError::validation(format!(
            "El atajo \u{201c}{normalized}\u{201d} ya esta asignado a otra accion."
        ))
        .with_detail("accelerator", normalized));
    }

    let registration = shortcuts::apply(&app, &state.shortcuts, &candidate);

    // Si el sistema rechazo justo el atajo que estabamos cambiando, volvemos al
    // anterior y no persistimos nada (§17.7).
    if let Some(rejected) = registration
        .rejected
        .iter()
        .find(|rejected| rejected.action == action.as_str())
    {
        let message = rejected.message.clone();
        shortcuts::apply(&app, &state.shortcuts, &previous);
        return Err(AppError::new(crate::errors::ErrorKind::Shortcut, message)
            .with_detail("accelerator", normalized));
    }

    settings.shortcuts = candidate.clone();
    settings_repo::save_shortcuts(&state.db, &candidate)?;
    events::emit(&app, events::SETTINGS_CHANGED, settings);

    Ok(ShortcutUpdate {
        settings: candidate,
        conflicts,
        registration,
        applied: normalized,
    })
}

#[tauri::command(async)]
pub fn reset_shortcuts(app: AppHandle, state: State<'_, AppState>) -> AppResult<ShortcutUpdate> {
    let defaults = ShortcutSettings::default();
    let registration = shortcuts::apply(&app, &state.shortcuts, &defaults);
    settings_repo::save_shortcuts(&state.db, &defaults)?;

    let settings = state.settings()?;
    events::emit(&app, events::SETTINGS_CHANGED, settings);

    Ok(ShortcutUpdate {
        applied: defaults
            .accelerator_for(ShortcutAction::ToggleOverlay)
            .unwrap_or_default()
            .to_string(),
        settings: defaults,
        conflicts: Vec::new(),
        registration,
    })
}

/// Catalogo de acciones con atajo, para que la interfaz no lo duplique.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutActionInfo {
    pub action: ShortcutAction,
    pub label: String,
    pub scope: ShortcutScope,
}

#[tauri::command(async)]
pub fn list_shortcut_actions(state: State<'_, AppState>) -> AppResult<Vec<ShortcutActionInfo>> {
    let settings = state.settings()?;
    Ok(ShortcutAction::ALL
        .into_iter()
        .map(|action| ShortcutActionInfo {
            scope: settings
                .shortcuts
                .bindings
                .iter()
                .find(|binding| binding.action == action)
                .map(|binding| binding.scope)
                .unwrap_or(ShortcutScope::Global),
            label: action.label().to_string(),
            action,
        })
        .collect())
}

/// Configura el arranque con el sistema a traves del plugin de autostart.
#[tauri::command(async)]
pub fn set_autostart(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> AppResult<bool> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    if let Err(error) = result {
        return Err(AppError::new(
            crate::errors::ErrorKind::Configuration,
            "No se pudo cambiar el inicio automatico con el sistema.",
        )
        .with_technical(error.to_string()));
    }

    let mut settings = state.settings()?;
    settings.general.start_with_system = enabled;
    settings_repo::save(&state.db, &settings)?;

    Ok(enabled)
}
