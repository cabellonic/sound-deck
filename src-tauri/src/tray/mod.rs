//! Icono y menu de la bandeja del sistema (§6).

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::errors::{AppError, AppResult, ErrorKind};
use crate::overlay;
use crate::state::AppState;

const ID_OPEN: &str = "tray_open";
const ID_OVERLAY: &str = "tray_overlay";
const ID_STOP: &str = "tray_stop";
const ID_SETTINGS: &str = "tray_settings";
const ID_QUIT: &str = "tray_quit";

/// Construye el icono de bandeja con su menu.
pub fn build(app: &AppHandle) -> AppResult<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, ID_OPEN, "Abrir soundboard", true, None::<&str>)?,
            &MenuItem::with_id(app, ID_OVERLAY, "Abrir overlay", true, None::<&str>)?,
            &MenuItem::with_id(
                app,
                ID_STOP,
                "Detener todos los sonidos",
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, ID_SETTINGS, "Configuracion", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, ID_QUIT, "Salir completamente", true, None::<&str>)?,
        ],
    )?;

    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        AppError::new(
            ErrorKind::Window,
            "No se encontro el icono de la aplicacion para la bandeja.",
        )
    })?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("Sound Deck")
        .menu(&menu)
        // El clic izquierdo abre la ventana; el menu se despliega con el derecho.
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = overlay::focus_main_window(tray.app_handle()) {
                    tracing::warn!(technical = ?error.technical, "no se pudo abrir la ventana desde la bandeja");
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let result = match event.id().as_ref() {
        ID_OPEN => overlay::focus_main_window(app),
        ID_OVERLAY => {
            let state = app.state::<AppState>();
            overlay::show(app, &state.overlay)
        }
        ID_STOP => {
            app.state::<AppState>().audio.stop_all();
            Ok(())
        }
        ID_SETTINGS => open_settings(app),
        ID_QUIT => {
            tracing::info!("salida solicitada desde la bandeja");
            app.state::<AppState>().audio.stop_all();
            app.exit(0);
            Ok(())
        }
        other => {
            tracing::debug!(id = other, "elemento de bandeja no manejado");
            Ok(())
        }
    };

    if let Err(error) = result {
        tracing::warn!(
            id = event.id().as_ref(),
            technical = ?error.technical,
            "fallo una accion de la bandeja"
        );
    }
}

/// Abre la ventana principal y le pide al frontend que muestre Ajustes.
fn open_settings(app: &AppHandle) -> AppResult<()> {
    overlay::focus_main_window(app)?;
    crate::events::emit(app, "open-settings", ());
    Ok(())
}
