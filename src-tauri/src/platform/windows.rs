//! Integracion con Win32 para recordar y restaurar la ventana activa (§16).
//!
//! Todas las llamadas Win32 de la aplicacion viven aqui: el resto del codigo
//! usa la API neutra de `platform`. No instalamos hooks de teclado globales ni
//! inyectamos nada en otros procesos; la unica razon por la que la tecla no
//! llega al juego es que el overlay tomo el foco.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow, ShowWindow,
    SW_RESTORE,
};

/// Handle opaco de la ventana que estaba activa antes de abrir el overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForegroundWindow(isize);

/// Captura la ventana en primer plano. Devuelve `None` si no hay ninguna.
pub fn capture_foreground_window() -> Option<ForegroundWindow> {
    // SAFETY: `GetForegroundWindow` no recibe punteros y puede devolver un HWND
    // nulo, caso que contemplamos explicitamente.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(ForegroundWindow(hwnd.0 as isize))
    }
}

/// Devuelve el foco a la ventana capturada.
///
/// Windows restringe quien puede robar el foco, asi que esto puede fallar de
/// forma legitima. Nunca entra en panico: si no se puede, se informa y listo (§16).
pub fn restore_foreground_window(window: ForegroundWindow) -> bool {
    let hwnd = HWND(window.0 as *mut core::ffi::c_void);

    // SAFETY: solo pasamos un HWND que validamos antes de usar. Todas estas
    // funciones aceptan handles invalidos devolviendo `FALSE`.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            tracing::debug!("la ventana anterior ya no existe; no se restaura el foco");
            return false;
        }

        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let restored = SetForegroundWindow(hwnd).as_bool();
        if !restored {
            tracing::debug!("el sistema no permitio devolver el foco a la ventana anterior");
        }
        restored
    }
}
