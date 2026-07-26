//! Control de la ventana overlay (§16).
//!
//! El overlay se crea al arrancar y queda oculto: mostrarlo es solo `show()` +
//! `set_focus()`, sin reconstruir la aplicacion. Antes de mostrarlo recordamos
//! cual era la ventana activa para devolverle el foco al cerrarlo.

use parking_lot::Mutex;
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

use crate::domain::settings::{GeneralSettings, OverlayPosition, OverlaySize};
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{self, OverlayVisibilityPayload};
use crate::platform::{self, ForegroundWindow};

pub const OVERLAY_LABEL: &str = "overlay";
pub const MAIN_LABEL: &str = "main";

/// Limites al redimensionar, en pixeles logicos.
///
/// Son una red de seguridad, no una opinion: quien lo achica hasta que solo se
/// distinguen las imagenes esta eligiendo eso. El alto lo decide el contenido,
/// asi que su limite solo esta para que la ventana nunca quede sin nada.
const MIN_SIZE: LogicalSize<f64> = LogicalSize::new(300.0, 200.0);
const MAX_SIZE: LogicalSize<f64> = LogicalSize::new(1100.0, 1100.0);

/// Posicion y tamano desde los que se entro al modo de ajuste, para poder volver.
#[derive(Clone, Copy)]
struct PreviousPlacement {
    position: Option<OverlayPosition>,
    size: Option<OverlaySize>,
}

/// Recuerda la ventana externa que tenia el foco antes de abrir el overlay.
#[derive(Default)]
pub struct OverlayState {
    previous_window: Mutex<Option<ForegroundWindow>>,
    placing: Mutex<Option<PreviousPlacement>>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Si el overlay se esta ajustando a mano. Mientras dure no se cierra al
    /// perder el foco ni despues de reproducir: se esta arrastrando.
    pub fn is_placing(&self) -> bool {
        self.placing.lock().is_some()
    }
}

/// Lo que quedo guardado al terminar de ajustar el overlay.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPlacement {
    pub position: OverlayPosition,
    pub size: OverlaySize,
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
    // Cerrarlo mientras se lo ajusta es cancelar el ajuste. Si no, el modo de
    // ajuste quedaria prendido y el overlay volveria a abrirse con la barra de
    // colocacion en vez de con los botones.
    if state.is_placing() {
        return cancel_placement(app, state);
    }

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

/// Abre el overlay para que el usuario lo arrastre y lo redimensione.
///
/// Mientras dure no se cierra al perder el foco: se lo va a estar arrastrando
/// con la ventana de Ajustes adelante.
pub fn begin_placement(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    let app_state = app.state::<crate::state::AppState>();
    let general = app_state.settings()?.general;
    *state.placing.lock() = Some(PreviousPlacement {
        position: general.overlay_position,
        size: general.overlay_size,
    });

    let window = overlay_window(app)?;
    if let Err(error) = place(&window, Some(&general)) {
        tracing::debug!(%error, "no se pudo colocar el overlay al empezar a moverlo");
    }

    // El overlay solo se puede redimensionar mientras se lo ajusta: el resto
    // del tiempo es una ventana fija que no se toca sin querer al jugar.
    window.set_min_size(Some(MIN_SIZE))?;
    window.set_max_size(Some(MAX_SIZE))?;
    window.set_resizable(true)?;

    window.show()?;
    window.set_focus()?;
    emit_placement(app, true);
    Ok(())
}

/// Guarda donde y de que tamano quedo el overlay, y sale del modo de ajuste.
///
/// `fit_height` es el alto que ocupa el contenido, medido por el propio
/// overlay: el ancho lo elige el usuario estirando la esquina, pero el alto se
/// ajusta solo para no guardar ni un hueco transparente de mas ni dejar el pie
/// cortado.
pub fn save_placement(
    app: &AppHandle,
    state: &OverlayState,
    fit_height: Option<u32>,
) -> AppResult<OverlayPlacement> {
    let window = overlay_window(app)?;
    let scale = window.scale_factor()?;

    if let Some(height) = fit_height {
        let height = f64::from(height).clamp(MIN_SIZE.height, MAX_SIZE.height);
        let width = window.inner_size()?.to_logical::<f64>(scale).width;
        window.set_size(LogicalSize::new(width, height))?;
    }

    let position = window.outer_position()?;
    let size = window.outer_size()?.to_logical::<f64>(scale);
    let placement = OverlayPlacement {
        position: OverlayPosition {
            x: position.x,
            y: position.y,
        },
        size: OverlaySize {
            width: size.width.round() as u32,
            height: size.height.round() as u32,
        },
    };

    let app_state = app.state::<crate::state::AppState>();
    let mut settings = app_state.settings()?;
    settings.general.overlay_position = Some(placement.position);
    settings.general.overlay_size = Some(placement.size);
    crate::database::settings::save_general(&app_state.db, &settings.general)?;

    finish_placement(app, state)?;
    crate::events::emit(app, crate::events::SETTINGS_CHANGED, settings);
    Ok(placement)
}

/// Sale del modo de ajuste dejando posicion y tamano como estaban.
pub fn cancel_placement(app: &AppHandle, state: &OverlayState) -> AppResult<()> {
    // Sin ajuste en curso no hay nada que restaurar: seguir adelante borraria
    // la posicion guardada, que es justo lo contrario de cancelar.
    let Some(previous) = state.placing.lock().take() else {
        return Ok(());
    };

    let app_state = app.state::<crate::state::AppState>();
    let mut settings = app_state.settings()?;
    if settings.general.overlay_position != previous.position
        || settings.general.overlay_size != previous.size
    {
        settings.general.overlay_position = previous.position;
        settings.general.overlay_size = previous.size;
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
    window.set_resizable(false)?;
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

/// Deja el overlay del tamano y en el lugar que corresponda antes de mostrarlo.
///
/// El tamano va primero porque el centrado depende de el. Una posicion elegida
/// a mano gana sobre todo lo demas; si no hay, se centra en el monitor activo
/// o, si esa opcion esta apagada, en el principal.
fn place(window: &WebviewWindow, general: Option<&GeneralSettings>) -> tauri::Result<()> {
    if let Some(size) = general.and_then(|general| general.overlay_size) {
        window.set_size(LogicalSize::new(
            f64::from(size.width),
            f64::from(size.height),
        ))?;
    }

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
