//! Arranque de la aplicacion: plugins, estado, bandeja, ventanas y atajos.

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::database::{settings as settings_repo, Database};
use crate::domain::settings::ShortcutAction;
use crate::errors::AppResult;
use crate::events::{self, NoticeLevel};
use crate::filesystem::AppPaths;
use crate::overlay::{self, MAIN_LABEL, OVERLAY_LABEL};
use crate::state::AppState;
use crate::{command_handlers, logging, shortcuts, tray};

/// Arranca Sound Deck.
pub fn run() {
    let builder = tauri::Builder::default()
        // Single instance debe ser el primer plugin: si ya hay una instancia,
        // este callback corre en ella y el proceso nuevo termina (§6).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!("segunda instancia detectada: se enfoca la ventana existente");
            if let Err(error) = overlay::focus_main_window(app) {
                tracing::warn!(technical = ?error.technical, "no se pudo enfocar la instancia existente");
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if !shortcuts::is_press(event.state()) {
                        return;
                    }
                    let state = app.state::<AppState>();
                    if let Some(action) = state.shortcuts.action_for(shortcut) {
                        handle_global_shortcut(app, action);
                    }
                })
                .build(),
        )
        .invoke_handler(command_handlers!())
        .setup(|app| {
            let handle = app.handle().clone();
            setup(&handle)?;
            Ok(())
        })
        .on_window_event(handle_window_event);

    builder
        .build(tauri::generate_context!())
        .expect("error al construir Sound Deck")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Detenemos el audio antes de cerrar para no dejar el
                // dispositivo tomado.
                if let Some(state) = app.try_state::<AppState>() {
                    state.audio.stop_all();
                }
            }
        });
}

/// Inicializacion que puede fallar. Un error aqui aborta el arranque de forma
/// explicita en lugar de dejar la aplicacion a medias.
fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let paths = AppPaths::resolve(app)?;

    // El logging se instala antes de tocar la base para que las migraciones y
    // el estado inicial queden registrados. El handle vive en el estado de
    // Tauri: si se soltara al terminar `setup`, se perderian los mensajes en vuelo.
    let log_handle = logging::init(&paths.logs_dir());

    tracing::info!(
        version = app.package_info().version.to_string(),
        data_dir = %paths.root().display(),
        "Sound Deck iniciado"
    );

    let db = Database::open(&paths.database_file())?;
    db.ensure_initial_state()?;

    let settings = settings_repo::load(&db)?;

    if let Some(handle) = log_handle {
        // Recien ahora sabemos que nivel quiere el usuario.
        handle.set_level(&settings.library.log_level);
        app.manage(handle);
    }

    // Limpiamos restos de descargas interrumpidas por un cierre anterior (§39).
    match paths.clean_temp() {
        Ok(removed) if removed > 0 => tracing::info!(removed, "temporales limpiados al arrancar"),
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(technical = ?error.technical, "no se pudieron limpiar los temporales")
        }
    }

    let http = crate::downloads::build_http_client();
    let state = AppState::new(app, db, paths, http);

    // Abrir el dispositivo guardado. Si falla, la aplicacion sigue viva: el
    // usuario puede elegir otro desde Ajustes (§18).
    match state.audio.open_device(
        settings.audio.output_device_id.as_deref(),
        settings.audio.output_device_name.as_deref(),
    ) {
        Ok((info, resolution)) => {
            tracing::info!(device = %info.name, "dispositivo de salida listo");
            if let Some(notice) = resolution.notice(settings.audio.output_device_name.as_deref()) {
                events::notify(app, NoticeLevel::Warning, notice);
            }
        }
        Err(error) => {
            tracing::error!(technical = ?error.technical, "no se pudo abrir ningun dispositivo de salida");
            events::notify(
                app,
                NoticeLevel::Error,
                "No se pudo abrir ningun dispositivo de salida. Revisa Ajustes > Audio.",
            );
        }
    }

    let shortcut_settings = settings.shortcuts.clone();
    app.manage(state);

    // Los atajos se registran despues de `manage` porque el handler necesita
    // acceder al estado.
    let state = app.state::<AppState>();
    let report = shortcuts::apply(app, &state.shortcuts, &shortcut_settings);
    for rejected in &report.rejected {
        events::notify(
            app,
            NoticeLevel::Warning,
            format!("{} ({}).", rejected.message, rejected.accelerator),
        );
    }

    tray::build(app)?;

    Ok(())
}

/// Ejecuta la accion asociada a un atajo global.
fn handle_global_shortcut(app: &AppHandle, action: ShortcutAction) {
    tracing::debug!(action = action.as_str(), "atajo global disparado");
    let state = app.state::<AppState>();

    let result: AppResult<()> = match action {
        ShortcutAction::ToggleOverlay => overlay::toggle(app, &state.overlay),
        ShortcutAction::StopAll => {
            state.audio.stop_all();
            Ok(())
        }
        // Las paginas se cambian dentro del overlay: reenviamos al frontend.
        ShortcutAction::PrevPage | ShortcutAction::NextPage => Ok(()),
    };

    if let Err(error) = result {
        tracing::warn!(
            action = action.as_str(),
            technical = ?error.technical,
            "fallo la accion del atajo global"
        );
    }

    shortcuts::emit_triggered(app, action);
}

/// Cierre a bandeja y cierre del overlay al perder el foco (§6, §16).
fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    let app = window.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    match (window.label(), event) {
        (MAIN_LABEL, WindowEvent::CloseRequested { api, .. }) => {
            let close_to_tray = state
                .settings()
                .map(|settings| settings.general.close_to_tray)
                .unwrap_or(true);

            if close_to_tray {
                // No se cierra de verdad: se oculta y sigue corriendo.
                api.prevent_close();
                if let Err(error) = window.hide() {
                    tracing::warn!(%error, "no se pudo ocultar la ventana principal");
                }
            }
        }
        (OVERLAY_LABEL, WindowEvent::Focused(false)) => {
            let close_on_blur = state
                .settings()
                .map(|settings| settings.general.close_overlay_on_blur)
                .unwrap_or(true);

            if close_on_blur {
                if let Err(error) = overlay::hide(app, &state.overlay) {
                    tracing::warn!(technical = ?error.technical, "no se pudo ocultar el overlay al perder foco");
                }
            }
        }
        _ => {}
    }
}
