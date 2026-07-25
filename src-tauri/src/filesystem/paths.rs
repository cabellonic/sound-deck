//! Rutas administradas por la aplicacion (§14).
//!
//! Toda escritura ocurre dentro del directorio de datos de la app; el frontend
//! nunca decide una ruta absoluta.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::errors::{AppError, AppResult, ErrorKind};

/// Layout de directorios de la aplicacion.
#[derive(Debug, Clone)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    /// Resuelve el directorio de datos de la app y crea la estructura si falta.
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let root = app.path().app_data_dir().map_err(|error| {
            AppError::new(
                ErrorKind::Filesystem,
                "No se pudo determinar la carpeta de datos de la aplicacion.",
            )
            .with_technical(error.to_string())
            .not_recoverable()
        })?;

        let paths = Self { root };
        paths.ensure_layout()?;
        Ok(paths)
    }

    /// Variante para tests: usa un directorio arbitrario como raiz.
    pub fn with_root(root: impl Into<PathBuf>) -> AppResult<Self> {
        let paths = Self { root: root.into() };
        paths.ensure_layout()?;
        Ok(paths)
    }

    fn ensure_layout(&self) -> AppResult<()> {
        for dir in [
            self.root.clone(),
            self.sounds_dir(),
            self.images_dir(),
            self.temp_dir(),
            self.logs_dir(),
            self.backups_dir(),
        ] {
            std::fs::create_dir_all(&dir).map_err(|error| {
                AppError::filesystem(format!(
                    "No se pudo crear la carpeta de datos {}.",
                    dir.display()
                ))
                .with_technical(error.to_string())
            })?;
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database_file(&self) -> PathBuf {
        self.root.join("database.sqlite")
    }

    /// Base que espera para reemplazar a la actual en el proximo arranque.
    ///
    /// Restaurar no puede sobrescribir el archivo que la aplicacion tiene
    /// abierto, asi que la copia elegida se deja aca y el reemplazo ocurre al
    /// arrancar, antes de abrir nada.
    pub fn database_restore_file(&self) -> PathBuf {
        self.root.join("database.restore.sqlite")
    }

    pub fn sounds_dir(&self) -> PathBuf {
        self.root.join("sounds")
    }

    /// Imagenes de los audios. Separadas de `sounds` para que el scope del
    /// protocolo `asset:` pueda abrir esta carpeta sin exponer los audios.
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("images")
    }

    pub fn temp_dir(&self) -> PathBuf {
        self.root.join("temp")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// Ruta definitiva de un sonido administrado, siempre `<uuid>.<ext>`.
    pub fn sound_file(&self, internal_filename: &str) -> PathBuf {
        self.sounds_dir().join(internal_filename)
    }

    /// Ruta definitiva de una imagen administrada, siempre `<uuid>.<ext>`.
    pub fn image_file(&self, internal_filename: &str) -> PathBuf {
        self.images_dir().join(internal_filename)
    }

    /// Verifica que una ruta este dentro del directorio administrado.
    /// Es la defensa contra path traversal para todo lo que venga del frontend.
    pub fn assert_managed(&self, candidate: &Path) -> AppResult<PathBuf> {
        self.assert_inside(self.sounds_dir(), candidate, "sonidos")
    }

    /// Igual que `assert_managed`, pero contra la carpeta de imagenes.
    pub fn assert_managed_image(&self, candidate: &Path) -> AppResult<PathBuf> {
        self.assert_inside(self.images_dir(), candidate, "imagenes")
    }

    fn assert_inside(&self, base: PathBuf, candidate: &Path, what: &str) -> AppResult<PathBuf> {
        let base = dunce::canonicalize(&base).map_err(|error| {
            AppError::filesystem(format!("No se pudo resolver la carpeta de {what}."))
                .with_technical(error.to_string())
        })?;
        let resolved = dunce::canonicalize(candidate).map_err(|error| {
            AppError::filesystem("El archivo indicado no existe o no es accesible.")
                .with_technical(format!("{}: {error}", candidate.display()))
        })?;

        if resolved.starts_with(&base) {
            Ok(resolved)
        } else {
            Err(
                AppError::validation("La ruta indicada esta fuera de la carpeta administrada.")
                    .with_technical(format!(
                        "ruta {} fuera de {}",
                        resolved.display(),
                        base.display()
                    )),
            )
        }
    }

    /// Ruta temporal unica para una descarga o importacion en curso.
    pub fn new_temp_file(&self, extension: &str) -> PathBuf {
        let extension = sanitize_extension(extension).unwrap_or_else(|| "part".to_string());
        self.temp_dir()
            .join(format!("{}.{extension}", uuid::Uuid::new_v4()))
    }

    /// Borra los restos de descargas o importaciones interrumpidas.
    pub fn clean_temp(&self) -> AppResult<u64> {
        let mut removed = 0;
        let entries = match std::fs::read_dir(self.temp_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error.into()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            match result {
                Ok(()) => removed += 1,
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "no se pudo borrar un temporal")
                }
            }
        }
        Ok(removed)
    }

    /// Tamano total ocupado por los audios administrados, en bytes.
    pub fn sounds_size_bytes(&self) -> u64 {
        let Ok(entries) = std::fs::read_dir(self.sounds_dir()) else {
            return 0;
        };
        entries
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .map(|metadata| metadata.len())
            .sum()
    }
}

/// Extensiones que la aplicacion acepta (§10).
pub const SUPPORTED_EXTENSIONS: [&str; 6] = ["mp3", "wav", "ogg", "flac", "m4a", "aac"];

/// Normaliza una extension a minusculas.
///
/// Rechaza (en lugar de "limpiar") cualquier cosa que no sea puramente
/// alfanumerica: silenciar `../../etc` convirtiendolo en `etc` esconderia un
/// intento de traversal en vez de delatarlo.
pub fn sanitize_extension(extension: &str) -> Option<String> {
    let candidate = extension.trim().trim_start_matches('.');

    if candidate.is_empty()
        || candidate.len() > 8
        || !candidate.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }

    Some(candidate.to_ascii_lowercase())
}

pub fn is_supported_extension(extension: &str) -> bool {
    sanitize_extension(extension)
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// Formatos de imagen aceptados para la miniatura de un audio.
/// Todos los decodifica el WebView sin ayuda nuestra.
pub const SUPPORTED_IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub fn is_supported_image_extension(extension: &str) -> bool {
    sanitize_extension(extension)
        .map(|ext| SUPPORTED_IMAGE_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// Sanitiza un nombre visible propuesto por el usuario o por un proveedor.
/// No se usa para construir rutas: el nombre en disco siempre es un UUID.
pub fn sanitize_display_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                ' '
            } else {
                c
            }
        })
        .collect();

    // Descartamos los tokens que son solo puntos (`..`, `.`): no aportan nada
    // legible y son el residuo tipico de una ruta. Un punto dentro de una
    // palabra ("Vol. 2") se conserva.
    let collapsed = cleaned
        .split_whitespace()
        .filter(|token| !token.chars().all(|c| c == '.'))
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = collapsed.trim();

    if trimmed.is_empty() {
        "Sin nombre".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

/// Nombre interno seguro: siempre `<uuid>.<ext>`. Ninguna parte proviene de
/// una URL remota ni de un header (§14).
pub fn build_internal_filename(extension: &str) -> String {
    let extension = sanitize_extension(extension).unwrap_or_else(|| "bin".to_string());
    format!("{}.{extension}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitiza_extensiones() {
        assert_eq!(sanitize_extension(".MP3").as_deref(), Some("mp3"));
        assert_eq!(sanitize_extension("wav").as_deref(), Some("wav"));
        assert_eq!(sanitize_extension("../../etc"), None);
        assert_eq!(sanitize_extension(""), None);
        assert_eq!(sanitize_extension("extensionlarguisima"), None);
    }

    #[test]
    fn reconoce_formatos_soportados() {
        assert!(is_supported_extension("mp3"));
        assert!(is_supported_extension(".FLAC"));
        assert!(!is_supported_extension("exe"));
        assert!(!is_supported_extension("mp3.exe"));
    }

    #[test]
    fn sanitiza_nombres_visibles() {
        assert_eq!(sanitize_display_name("  hola   mundo "), "hola mundo");
        assert_eq!(sanitize_display_name("../../etc/passwd"), "etc passwd");
        assert_eq!(sanitize_display_name("a:b*c?d"), "a b c d");
        assert_eq!(sanitize_display_name("   "), "Sin nombre");
        assert_eq!(sanitize_display_name("..."), "Sin nombre");
        // Un punto dentro de una palabra es legitimo y se conserva.
        assert_eq!(sanitize_display_name("Vol. 2 - intro"), "Vol. 2 - intro");
        // Unicode legitimo se conserva.
        assert_eq!(
            sanitize_display_name("Canción ñandú 🎵"),
            "Canción ñandú 🎵"
        );
    }

    #[test]
    fn nombre_interno_es_uuid_con_extension() {
        let name = build_internal_filename("MP3");
        assert!(name.ends_with(".mp3"));
        assert_eq!(name.len(), 36 + 4);
        // Una extension hostil nunca produce una ruta relativa.
        let hostil = build_internal_filename("../../evil");
        assert!(!hostil.contains('/') && !hostil.contains('\\'));
    }

    #[test]
    fn assert_managed_rechaza_rutas_externas() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_root(temp.path()).unwrap();

        let interno = paths.sound_file("valido.mp3");
        std::fs::write(&interno, b"data").unwrap();
        assert!(paths.assert_managed(&interno).is_ok());

        let externo = temp.path().join("fuera.mp3");
        std::fs::write(&externo, b"data").unwrap();
        assert!(paths.assert_managed(&externo).is_err());

        // Un intento explicito de traversal tambien falla.
        let traversal = paths.sounds_dir().join("..").join("fuera.mp3");
        assert!(paths.assert_managed(&traversal).is_err());
    }

    #[test]
    fn las_carpetas_de_audio_e_imagenes_no_se_mezclan() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_root(temp.path()).unwrap();

        let audio = paths.sound_file("a.mp3");
        let imagen = paths.image_file("a.png");
        std::fs::write(&audio, b"data").unwrap();
        std::fs::write(&imagen, b"data").unwrap();

        assert!(paths.assert_managed(&audio).is_ok());
        assert!(paths.assert_managed_image(&imagen).is_ok());

        // Cada validador acota su propia carpeta: el scope de `asset:` puede
        // abrir las imagenes sin que eso alcance para leer los audios.
        assert!(paths.assert_managed(&imagen).is_err());
        assert!(paths.assert_managed_image(&audio).is_err());
    }

    #[test]
    fn reconoce_formatos_de_imagen() {
        assert!(is_supported_image_extension("png"));
        assert!(is_supported_image_extension(".JPEG"));
        assert!(is_supported_image_extension("webp"));
        assert!(!is_supported_image_extension("svg"));
        assert!(!is_supported_image_extension("exe"));
        assert!(!is_supported_image_extension("mp3"));
    }

    #[test]
    fn crea_el_layout_completo() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_root(temp.path()).unwrap();
        assert!(paths.sounds_dir().is_dir());
        assert!(paths.images_dir().is_dir());
        assert!(paths.temp_dir().is_dir());
        assert!(paths.logs_dir().is_dir());
        assert!(paths.backups_dir().is_dir());
    }

    #[test]
    fn limpia_temporales() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_root(temp.path()).unwrap();
        std::fs::write(paths.temp_dir().join("a.part"), b"x").unwrap();
        std::fs::write(paths.temp_dir().join("b.part"), b"x").unwrap();

        assert_eq!(paths.clean_temp().unwrap(), 2);
        assert_eq!(paths.clean_temp().unwrap(), 0);
    }
}
