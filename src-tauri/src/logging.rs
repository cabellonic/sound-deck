//! Configuracion de logs (§35).
//!
//! Los logs van a un archivo rotativo diario en `AppData/.../logs` y, en
//! compilaciones de desarrollo, tambien a la consola.
//!
//! El subscriber se instala **antes** de abrir la base de datos para que las
//! migraciones y el estado inicial queden registrados. Como el nivel elegido por
//! el usuario vive justamente en esa base, el filtro es recargable: arranca en
//! `info` y se ajusta en cuanto la configuracion esta disponible.
//!
//! Nunca se registran API keys ni contenido binario.

use std::path::Path;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{reload, EnvFilter};

/// Handle del sistema de logs.
///
/// Debe mantenerse vivo durante toda la ejecucion: al soltarlo se pierden los
/// mensajes que todavia no se escribieron a disco.
pub struct LogHandle {
    _guard: tracing_appender::non_blocking::WorkerGuard,
    /// Cambia el nivel sin reiniciar. Devuelve `false` si el subscriber murio.
    set_filter: Box<dyn Fn(&str) -> bool + Send + Sync>,
}

impl LogHandle {
    /// Aplica el nivel configurado por el usuario.
    pub fn set_level(&self, level: &str) {
        let level = normalize_level(level);
        let directive = format!("sound_deck_lib={level},sound_deck={level}");

        if (self.set_filter)(&directive) {
            tracing::debug!(level, "nivel de logs aplicado");
        } else {
            tracing::warn!(level, "no se pudo cambiar el nivel de logs");
        }
    }
}

/// Instala el subscriber. Un fallo aca no debe impedir que la app arranque.
pub fn init(logs_dir: &Path) -> Option<LogHandle> {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("sound-deck")
        .filename_suffix("log")
        .max_log_files(7)
        .build(logs_dir)
        .map_err(|error| eprintln!("no se pudo abrir el archivo de log: {error}"))
        .ok()?;

    let (writer, guard) = tracing_appender::non_blocking(appender);

    // `SOUND_DECK_LOG` gana sobre la configuracion guardada: sirve para depurar
    // un arranque que falla antes de poder leer la base.
    let forced = std::env::var("SOUND_DECK_LOG").ok();
    let initial = forced
        .as_deref()
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new("sound_deck_lib=info,sound_deck=info"));

    let (filter, reload_handle) = reload::Layer::new(initial);
    let locked = forced.is_some();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(tracing_subscriber::fmt::layer().with_target(true));

    if registry.try_init().is_err() {
        // Ya habia un subscriber (por ejemplo en tests): no es un error fatal.
        return None;
    }

    Some(LogHandle {
        _guard: guard,
        set_filter: Box::new(move |directive: &str| {
            if locked {
                return true;
            }
            match EnvFilter::try_new(directive) {
                Ok(filter) => reload_handle.reload(filter).is_ok(),
                Err(_) => false,
            }
        }),
    })
}

/// Nivel valido para `EnvFilter`, con fallback seguro.
pub fn normalize_level(level: &str) -> &'static str {
    match level.trim().to_ascii_lowercase().as_str() {
        "error" => "error",
        "warn" => "warn",
        "debug" => "debug",
        "trace" => "trace",
        _ => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_niveles_desconocidos_a_info() {
        assert_eq!(normalize_level("DEBUG"), "debug");
        assert_eq!(normalize_level(" warn "), "warn");
        assert_eq!(normalize_level("verboso"), "info");
        assert_eq!(normalize_level(""), "info");
    }

    #[test]
    fn crea_el_archivo_de_log_en_el_directorio_indicado() {
        let dir = tempfile::tempdir().unwrap();

        // `init` puede devolver `None` si otro test ya instalo un subscriber
        // global; en ese caso solo verificamos que no entre en panico.
        if init(dir.path()).is_some() {
            tracing::info!("mensaje de prueba");
            let archivos = std::fs::read_dir(dir.path()).unwrap().count();
            assert_eq!(archivos, 1, "debe crearse un archivo de log diario");
        }
    }
}
