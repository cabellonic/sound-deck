//! Capa de plataforma.
//!
//! Cada requisito que depende del sistema operativo se resuelve detras de esta
//! interfaz (§4.20). En plataformas sin implementacion, las funciones degradan
//! a un no-op silencioso en lugar de fallar.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{capture_foreground_window, restore_foreground_window, ForegroundWindow};

#[cfg(not(windows))]
mod fallback;

#[cfg(not(windows))]
pub use fallback::{capture_foreground_window, restore_foreground_window, ForegroundWindow};

/// Si la plataforma actual puede devolver el foco a la aplicacion anterior.
/// La interfaz lo usa para no prometer algo que no va a pasar.
pub const fn supports_focus_restore() -> bool {
    cfg!(windows)
}
