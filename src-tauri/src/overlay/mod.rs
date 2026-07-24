//! Control de la ventana overlay (§16).
//!
//! El overlay se crea al arrancar y queda oculto: mostrarlo es solo `show()` +
//! `set_focus()`, sin reconstruir la aplicacion. Antes de mostrarlo recordamos
//! cual era la ventana activa para devolverle el foco al cerrarlo.

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{self, OverlayVisibilityPayload};
use crate::platform::{self, ForegroundWindow};

pub const OVERLAY_LABEL: &str = "overlay";
pub const MAIN_LABEL: &str = "main";

/// Recuerda la ventana externa que tenia el foco antes de abrir el overlay.
#[derive(Default)]
pub struct OverlayState {
    previous_window: Mutex<Option<ForegroundWindow>>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self::default()
    }
}

fn overlay_window(app: &AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(OVERLAY_LABEL).ok_or_else(|| {
        AppError::new(
            ErrorKind::Window,
            "La ventana del overlay no esta disponible. Reinicia la aplicacion.",
        )
    })
}

/// Muestra el overlay y le da el foco.
pub fn show(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    // Se captura antes de mostrar: despues, la ventana activa ya seria el overlay.
    *state.previous_window.lock() = platform::capture_foreground_window();

    let window = overlay_window(app)?;

    if let Err(error) = center_on_active_monitor(&window) {
        tracing::debug!(%error, "no se pudo centrar el overlay en el monitor activo");
    }

    window.show()?;
    window.set_always_on_top(true)?;
    window.set_focus()?;

    events::emit(
        app,
        events::OVERLAY_VISIBILITY_CHANGED,
        OverlayVisibilityPayload { visible: true },
    );
    Ok(())
}

/// Oculta el overlay y devuelve el foco a la aplicacion anterior si se puede.
pub fn hide(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    let window = overlay_window(app)?;
    let was_visible = window.is_visible().unwrap_or(false);
    window.hide()?;

    if let Some(previous) = state.previous_window.lock().take() {
        // Un fallo aqui no es un error para el usuario: el sistema puede
        // rechazar legitimamente el cambio de foco.
        let restored = platform::restore_foreground_window(previous);
        tracing::debug!(
            restored,
            "intento de devolver el foco a la ventana anterior"
        );
    }

    if was_visible {
        events::emit(
            app,
            events::OVERLAY_VISIBILITY_CHANGED,
            OverlayVisibilityPayload { visible: false },
        );
    }
    Ok(())
}

/// Alterna la visibilidad. Es lo que dispara el atajo global.
pub fn toggle(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    let window = overlay_window(app)?;
    if window.is_visible().unwrap_or(false) {
        hide(app, state)
    } else {
        show(app, state)
    }
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(OVERLAY_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// Centra el overlay en el monitor donde esta el cursor, para que aparezca en
/// la pantalla que el usuario esta mirando (§20 "abrir overlay en monitor activo").
fn center_on_active_monitor(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) =
        window.monitor_from_point(window.cursor_position()?.x, window.cursor_position()?.y)?
    else {
        return window.center();
    };

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size()?;

    window.set_position(tauri::PhysicalPosition::new(
        monitor_position.x + (monitor_size.width as i32 - window_size.width as i32) / 2,
        monitor_position.y + (monitor_size.height as i32 - window_size.height as i32) / 2,
    ))?;
    Ok(())
}

/// Trae la ventana principal al frente (bandeja, single instance, onboarding).
pub fn focus_main_window(app: &AppHandle) -> AppResult<()> {
    let window = app.get_webview_window(MAIN_LABEL).ok_or_else(|| {
        AppError::new(
            ErrorKind::Window,
            "La ventana principal no esta disponible.",
        )
    })?;

    window.show()?;
    if window.is_minimized().unwrap_or(false) {
        window.unminimize()?;
    }
    window.set_focus()?;
    Ok(())
}
