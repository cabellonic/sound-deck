//! Control de la ventana overlay (§16).
//!
//! El overlay se crea al arrancar y queda oculto: mostrarlo es solo `show()` +
//! `set_focus()`, sin reconstruir la aplicacion. Antes de mostrarlo recordamos
//! cual era la ventana activa para devolverle el foco al cerrarlo.

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::domain::settings::{GeneralSettings, OverlayPosition};
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{self, OverlayVisibilityPayload};
use crate::platform::{self, ForegroundWindow};

pub const OVERLAY_LABEL: &str = "overlay";
pub const MAIN_LABEL: &str = "main";

/// Recuerda la ventana externa que tenia el foco antes de abrir el overlay.
#[derive(Default)]
pub struct OverlayState {
    previous_window: Mutex<Option<ForegroundWindow>>,
    /// Posicion desde la que se entro al modo de colocacion, para poder volver.
    placing: Mutex<Option<Option<OverlayPosition>>>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Si el overlay se esta colocando a mano. Mientras dure no se cierra al
    /// perder el foco ni despues de reproducir: se esta arrastrando.
    pub fn is_placing(&self) -> bool {
        self.placing.lock().is_some()
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
    let general = app
        .state::<crate::state::AppState>()
        .settings()
        .map(|settings| settings.general)
        .ok();

    if let Err(error) = place(&window, general.as_ref()) {
        tracing::debug!(%error, "no se pudo colocar el overlay");
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

/// Abre el overlay para que el usuario lo arrastre a donde quiera.
///
/// Mientras dure no se cierra al perder el foco: se lo va a estar arrastrando
/// con la ventana de Ajustes adelante.
pub fn begin_placement(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    let app_state = app.state::<crate::state::AppState>();
    let previous = app_state.settings()?.general.overlay_position;
    *state.placing.lock() = Some(previous);

    let window = overlay_window(app)?;
    if let Err(error) = place(&window, Some(&app_state.settings()?.general)) {
        tracing::debug!(%error, "no se pudo colocar el overlay al empezar a moverlo");
    }

    window.show()?;
    window.set_focus()?;
    emit_placement(app, true);
    Ok(())
}

/// Guarda donde quedo el overlay y sale del modo de colocacion.
pub fn save_placement(app: &AppHandle, state: &OverlayState) -> AppResult<OverlayPosition> {
    let window = overlay_window(app)?;
    let position = window.outer_position()?;
    let position = OverlayPosition {
        x: position.x,
        y: position.y,
    };

    let app_state = app.state::<crate::state::AppState>();
    let mut settings = app_state.settings()?;
    settings.general.overlay_position = Some(position);
    crate::database::settings::save_general(&app_state.db, &settings.general)?;

    finish_placement(app, state)?;
    crate::events::emit(app, crate::events::SETTINGS_CHANGED, settings);
    Ok(position)
}

/// Sale del modo de colocacion dejando la posicion como estaba.
pub fn cancel_placement(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    let previous = state.placing.lock().take().flatten();

    let app_state = app.state::<crate::state::AppState>();
    let mut settings = app_state.settings()?;
    if settings.general.overlay_position != previous {
        settings.general.overlay_position = previous;
        crate::database::settings::save_general(&app_state.db, &settings.general)?;
        crate::events::emit(app, crate::events::SETTINGS_CHANGED, settings);
    }

    finish_placement(app, state)
}

/// Vuelve al centrado automatico, olvidando la posicion elegida.
pub fn clear_placement(app: &AppHandle) -> AppResult<()> {
    let app_state = app.state::<crate::state::AppState>();
    let mut settings = app_state.settings()?;
    settings.general.overlay_position = None;
    crate::database::settings::save_general(&app_state.db, &settings.general)?;
    crate::events::emit(app, crate::events::SETTINGS_CHANGED, settings);
    Ok(())
}

fn finish_placement(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    *state.placing.lock() = None;
    emit_placement(app, false);

    let window = overlay_window(app)?;
    window.hide()?;
    crate::events::emit(
        app,
        events::OVERLAY_VISIBILITY_CHANGED,
        OverlayVisibilityPayload { visible: false },
    );

    // Colocar el overlay se hace desde Ajustes: el foco vuelve ahi, no a lo que
    // hubiera antes de abrir la ventana principal.
    focus_main_window(app)
}

fn emit_placement(app: &AppHandle, placing: bool) {
    crate::events::emit(
        app,
        events::OVERLAY_PLACEMENT_CHANGED,
        PlacementPayload { placing },
    );
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlacementPayload {
    placing: bool,
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(OVERLAY_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// Coloca el overlay donde corresponda antes de mostrarlo.
///
/// Una posicion elegida a mano gana sobre todo lo demas; si no hay, se centra
/// en el monitor activo o, si esa opcion esta apagada, en el principal.
fn place(window: &WebviewWindow, general: Option<&GeneralSettings>) -> tauri::Result<()> {
    match general.and_then(|general| general.overlay_position) {
        Some(position) => {
            window.set_position(tauri::PhysicalPosition::new(position.x, position.y))?;
            ensure_on_screen(window)
        }
        None if general.is_none_or(|general| general.overlay_on_active_monitor) => {
            center_on_active_monitor(window)
        }
        None => window.center(),
    }
}

/// Devuelve el overlay a la pantalla si quedo fuera de todo monitor.
///
/// Pasa al desenchufar el monitor donde estaba colocado: sin esto, el overlay
/// se abriria en coordenadas que ya no existen y el usuario no lo veria mas.
fn ensure_on_screen(window: &WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    if window
        .monitor_from_point(position.x.into(), position.y.into())?
        .is_some()
    {
        return Ok(());
    }

    tracing::info!("la posicion guardada del overlay quedo fuera de pantalla: se centra");
    window.center()
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

    // Windows le aplica al primer ShowWindow de un proceso el `wShowWindow` que
    // eligio quien lo lanzo. Si ese padre pidio arrancar oculto, el primer
    // `show()` se descarta en silencio y la ventana no aparece nunca, porque
    // desde §6 nace con `visible: false`. El segundo ya no se descarta.
    if !window.is_visible().unwrap_or(true) {
        window.show()?;
    }

    if window.is_minimized().unwrap_or(false) {
        window.unminimize()?;
    }
    window.set_focus()?;
    Ok(())
}
