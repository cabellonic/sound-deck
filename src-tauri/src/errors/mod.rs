//! Errores de dominio de la aplicacion.
//!
//! Todo comando Tauri devuelve `Result<T, AppError>`. `AppError` se serializa
//! hacia el frontend como `{ code, message, recoverable, details }` (ver §29 del
//! prompt maestro). Los detalles tecnicos crudos van a los logs, nunca al usuario.

use std::collections::BTreeMap;

use serde::{Serialize, Serializer};

/// Categoria del error. El `code` que ve el frontend deriva de aqui.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Database,
    Filesystem,
    InvalidAudio,
    AudioDevice,
    Playback,
    Network,
    Provider,
    Download,
    Shortcut,
    Window,
    Validation,
    Configuration,
    UnsupportedPlatform,
    NotFound,
}

impl ErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::Database => "DATABASE",
            ErrorKind::Filesystem => "FILESYSTEM",
            ErrorKind::InvalidAudio => "INVALID_AUDIO",
            ErrorKind::AudioDevice => "AUDIO_DEVICE",
            ErrorKind::Playback => "PLAYBACK",
            ErrorKind::Network => "NETWORK",
            ErrorKind::Provider => "PROVIDER",
            ErrorKind::Download => "DOWNLOAD",
            ErrorKind::Shortcut => "SHORTCUT",
            ErrorKind::Window => "WINDOW",
            ErrorKind::Validation => "VALIDATION",
            ErrorKind::Configuration => "CONFIGURATION",
            ErrorKind::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            ErrorKind::NotFound => "NOT_FOUND",
        }
    }

    /// Si el usuario puede razonablemente reintentar o corregir la situacion.
    fn default_recoverable(self) -> bool {
        !matches!(self, ErrorKind::UnsupportedPlatform)
    }
}

/// Error visible para el usuario, con contexto tecnico separado para los logs.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub kind: ErrorKind,
    /// Mensaje accionable en espanol, apto para mostrarse tal cual.
    pub message: String,
    /// Contexto tecnico. Se registra en logs; nunca se muestra al usuario.
    pub technical: Option<String>,
    pub recoverable: bool,
    pub details: BTreeMap<String, String>,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            technical: None,
            recoverable: kind.default_recoverable(),
            details: BTreeMap::new(),
        }
    }

    /// Adjunta contexto tecnico que solo va a los logs.
    pub fn with_technical(mut self, technical: impl Into<String>) -> Self {
        self.technical = Some(technical.into());
        self
    }

    /// Adjunta un dato estructurado que el frontend puede usar para actuar
    /// (por ejemplo el id del sonido roto para ofrecer "quitar asignacion").
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn not_recoverable(mut self) -> Self {
        self.recoverable = false;
        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Database, message)
    }

    pub fn filesystem(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Filesystem, message)
    }
}

/// Representacion enviada al frontend. Coincide con `AppError` en TypeScript.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedError<'a> {
    code: &'static str,
    message: &'a str,
    recoverable: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    details: &'a BTreeMap<String, String>,
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // El contexto tecnico se registra aqui y no cruza el IPC.
        if let Some(technical) = &self.technical {
            tracing::error!(code = self.kind.code(), technical, "{}", self.message);
        } else {
            tracing::warn!(code = self.kind.code(), "{}", self.message);
        }

        SerializedError {
            code: self.kind.code(),
            message: &self.message,
            recoverable: self.recoverable,
            details: &self.details,
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        AppError::database("No se pudo completar una operacion en la base de datos.")
            .with_technical(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        let message = match value.kind() {
            std::io::ErrorKind::NotFound => "No se encontro el archivo solicitado.",
            std::io::ErrorKind::PermissionDenied => {
                "El sistema denego el acceso al archivo o carpeta."
            }
            _ => "Ocurrio un error de acceso al sistema de archivos.",
        };
        AppError::filesystem(message).with_technical(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::new(
            ErrorKind::Configuration,
            "Un valor de configuracion guardado no pudo interpretarse.",
        )
        .with_technical(value.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        AppError::new(ErrorKind::Window, "La operacion sobre la ventana fallo.")
            .with_technical(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_codigo_y_mensaje_sin_contexto_tecnico() {
        let error = AppError::validation("El nombre no puede estar vacio.")
            .with_technical("campo `name` recibido con longitud 0")
            .with_detail("field", "name");

        let json = serde_json::to_value(&error).expect("debe serializar");

        assert_eq!(json["code"], "VALIDATION");
        assert_eq!(json["message"], "El nombre no puede estar vacio.");
        assert_eq!(json["recoverable"], true);
        assert_eq!(json["details"]["field"], "name");
        // El contexto tecnico jamas debe cruzar el IPC.
        assert!(json.get("technical").is_none());
    }

    #[test]
    fn omite_details_cuando_esta_vacio() {
        let error = AppError::not_found("No existe la pagina.");
        let json = serde_json::to_value(&error).expect("debe serializar");
        assert!(json.get("details").is_none());
    }
}
