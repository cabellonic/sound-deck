//! Inicio con el sistema (§6, §43).
//!
//! El estado real no vive en la base: vive en el registro de Windows o en el
//! LaunchAgent/`.desktop` del sistema. La base solo guarda una copia para que
//! Ajustes pueda pintar el interruptor sin bloquear. Cuando las dos difieren
//! manda el sistema: si el usuario desactivo Sound Deck desde el Administrador
//! de tareas, la aplicacion no tiene por que volver a meterse ahi sola.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::database::{settings as settings_repo, Database};
use crate::domain::settings::GeneralSettings;
use crate::errors::{AppError, AppResult, ErrorKind};

/// Argumento con el que el sistema lanza Sound Deck al iniciar sesion. Es lo
/// unico que distingue ese arranque de uno hecho a mano por el usuario.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Si este proceso lo lanzo el arranque del sistema.
pub fn launched_by_system() -> bool {
    contains_autostart_arg(std::env::args())
}

/// Separado de `launched_by_system` para poder probar el parseo sin depender
/// de como se invoco el proceso de test.
pub fn contains_autostart_arg<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // Se saltea `argv[0]`: es la ruta del ejecutable, no un argumento.
    args.into_iter()
        .skip(1)
        .any(|arg| arg.as_ref() == AUTOSTART_ARG)
}

/// Activa o desactiva el arranque con el sistema. Solo toca el sistema; la
/// persistencia queda a cargo de quien llama.
pub fn set_enabled(app: &AppHandle, enabled: bool) -> AppResult<()> {
    // En desarrollo lo que se registra es el binario de `target/debug`, que
    // puede desaparecer en el proximo `cargo clean`.
    #[cfg(debug_assertions)]
    if enabled {
        tracing::warn!("inicio automatico activado sobre un binario de desarrollo");
    }

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    result.map_err(|error| {
        AppError::new(
            ErrorKind::Configuration,
            "No se pudo cambiar el inicio automatico con el sistema.",
        )
        .with_technical(error.to_string())
    })
}

/// Lo que dice el sistema. `None` si no se pudo consultar: en ese caso no
/// sabemos nada y conviene no tocar lo guardado.
pub fn system_state(app: &AppHandle) -> Option<bool> {
    match app.autolaunch().is_enabled() {
        Ok(enabled) => Some(enabled),
        Err(error) => {
            tracing::warn!(%error, "no se pudo consultar el inicio automatico del sistema");
            None
        }
    }
}

/// Decide si hay que corregir el valor guardado. Devuelve `Some(valor)` cuando
/// la base quedo desactualizada respecto del sistema.
pub fn reconciled_value(stored: bool, system: Option<bool>) -> Option<bool> {
    match system {
        Some(actual) if actual != stored => Some(actual),
        _ => None,
    }
}

/// Alinea la configuracion guardada con el estado real del sistema al arrancar.
///
/// Corrige el caso tipico: el usuario desactivo el inicio automatico desde el
/// Administrador de tareas y Ajustes seguia mostrando el interruptor prendido.
pub fn reconcile(app: &AppHandle, db: &Database, general: &mut GeneralSettings) {
    let Some(actual) = reconciled_value(general.start_with_system, system_state(app)) else {
        return;
    };

    tracing::info!(
        actual,
        "el inicio automatico cambio fuera de la aplicacion: se actualiza lo guardado"
    );
    general.start_with_system = actual;

    if let Err(error) = settings_repo::save_general(db, general) {
        tracing::warn!(
            technical = ?error.technical,
            "no se pudo guardar el estado real del inicio automatico"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_el_argumento_de_arranque_automatico() {
        assert!(contains_autostart_arg(["sound-deck.exe", "--autostart"]));
        assert!(contains_autostart_arg([
            "sound-deck.exe",
            "--otro",
            "--autostart"
        ]));
    }

    #[test]
    fn un_arranque_normal_no_trae_el_argumento() {
        assert!(!contains_autostart_arg(["sound-deck.exe"]));
        assert!(!contains_autostart_arg(["sound-deck.exe", "--verbose"]));
    }

    #[test]
    fn el_ejecutable_no_cuenta_como_argumento() {
        // Un ejecutable que por lo que sea se llame igual que el argumento no
        // debe hacer que la aplicacion arranque escondida.
        assert!(!contains_autostart_arg(["--autostart"]));
    }

    #[test]
    fn manda_el_sistema_cuando_difiere_de_lo_guardado() {
        assert_eq!(reconciled_value(true, Some(false)), Some(false));
        assert_eq!(reconciled_value(false, Some(true)), Some(true));
    }

    #[test]
    fn no_se_toca_nada_si_coinciden_o_no_se_pudo_consultar() {
        assert_eq!(reconciled_value(true, Some(true)), None);
        assert_eq!(reconciled_value(false, Some(false)), None);
        assert_eq!(reconciled_value(true, None), None);
        assert_eq!(reconciled_value(false, None), None);
    }
}
