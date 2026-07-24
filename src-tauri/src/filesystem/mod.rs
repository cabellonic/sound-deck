//! Acceso al sistema de archivos administrado por la aplicacion.

pub mod audio_file;
pub mod image_file;
pub mod paths;

pub use audio_file::{probe_audio_file, AudioProbe};
pub use image_file::{probe_image_file, ImageProbe};
pub use paths::AppPaths;

use std::path::Path;

use crate::errors::{AppError, AppResult};

/// Mueve un archivo temporal a su destino definitivo de forma atomica cuando el
/// sistema lo permite. Si origen y destino estan en volumenes distintos,
/// `rename` falla y hacemos copy+remove como fallback (§14.12).
pub fn move_into_place(from: &Path, to: &Path) -> AppResult<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            std::fs::copy(from, to).map_err(|copy_error| {
                AppError::filesystem("No se pudo guardar el archivo de audio en la biblioteca.")
                    .with_technical(format!(
                        "rename: {rename_error}; copy {} -> {}: {copy_error}",
                        from.display(),
                        to.display()
                    ))
            })?;
            if let Err(error) = std::fs::remove_file(from) {
                tracing::warn!(path = %from.display(), %error, "no se pudo borrar el temporal tras copiar");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mueve_creando_el_directorio_destino() {
        let dir = tempfile::tempdir().unwrap();
        let origen = dir.path().join("origen.bin");
        let destino = dir.path().join("sub").join("destino.bin");
        std::fs::write(&origen, b"contenido").unwrap();

        move_into_place(&origen, &destino).unwrap();

        assert!(!origen.exists());
        assert_eq!(std::fs::read(&destino).unwrap(), b"contenido");
    }
}
