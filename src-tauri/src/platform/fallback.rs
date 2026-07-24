//! Implementacion neutra para plataformas donde todavia no resolvemos la
//! restauracion de foco (Linux/X11-Wayland, macOS).
//!
//! El overlay sigue funcionando: se abre, toma foco y reproduce. Lo unico que
//! no ocurre es la devolucion explicita del foco a la aplicacion anterior, que
//! en esas plataformas suele manejar el propio gestor de ventanas.

/// Handle opaco. En estas plataformas no guardamos nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForegroundWindow;

pub fn capture_foreground_window() -> Option<ForegroundWindow> {
    None
}

pub fn restore_foreground_window(_window: ForegroundWindow) -> bool {
    false
}
