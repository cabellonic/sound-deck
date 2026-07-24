//! Validacion de las imagenes que el usuario asigna a un audio.
//!
//! Misma regla que con el audio (§10, §14): no confiamos en la extension del
//! archivo, olfateamos el contenido. La diferencia es que aca no hace falta un
//! decodificador propio, porque el que va a mostrar la imagen es el WebView:
//! nos alcanza con aceptar solo formatos que sabemos que sabe abrir.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::errors::{AppError, AppResult, ErrorKind};

use super::audio_file::format_bytes;

/// Formato de imagen detectado a partir de los bytes iniciales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedImage {
    Png,
    Jpeg,
    WebP,
}

impl DetectedImage {
    pub fn extension(self) -> &'static str {
        match self {
            DetectedImage::Png => "png",
            DetectedImage::Jpeg => "jpg",
            DetectedImage::WebP => "webp",
        }
    }
}

/// Olfatea el formato por la cabecera. `None` si no reconoce nada.
pub fn sniff_image(header: &[u8]) -> Option<DetectedImage> {
    if header.len() < 12 {
        return None;
    }

    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(DetectedImage::Png);
    }
    if header.starts_with(b"\xff\xd8\xff") {
        return Some(DetectedImage::Jpeg);
    }
    if header.starts_with(b"RIFF") && &header[8..12] == b"WEBP" {
        return Some(DetectedImage::WebP);
    }

    None
}

/// Tamano maximo de una imagen de boton. Es una miniatura de 3x3 en pantalla:
/// mas que esto no mejora nada y solo infla la carpeta de datos.
pub const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ImageProbe {
    pub format: DetectedImage,
    /// Extension final que usara el archivo administrado.
    pub extension: String,
    pub size_bytes: u64,
}

/// Valida que el archivo sea una imagen que el WebView pueda mostrar.
pub fn probe_image_file(path: &Path) -> AppResult<ImageProbe> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        AppError::filesystem("La imagen no existe o no se puede leer.")
            .with_technical(format!("{}: {error}", path.display()))
    })?;

    if !metadata.is_file() {
        return Err(AppError::validation(
            "La ruta indicada no corresponde a un archivo.",
        ));
    }

    let size_bytes = metadata.len();
    if size_bytes == 0 {
        return Err(AppError::new(
            ErrorKind::Validation,
            "El archivo de imagen esta vacio.",
        ));
    }
    if size_bytes > MAX_IMAGE_BYTES {
        return Err(AppError::new(
            ErrorKind::Validation,
            format!(
                "La imagen pesa {} y supera el limite de {}.",
                format_bytes(size_bytes),
                format_bytes(MAX_IMAGE_BYTES)
            ),
        ));
    }

    let mut file = File::open(path)?;
    let mut header = [0u8; 16];
    let read = file.read(&mut header)?;
    let format = sniff_image(&header[..read]).ok_or_else(|| {
        AppError::new(
            ErrorKind::Validation,
            "El archivo no parece ser una imagen PNG, JPG o WebP.",
        )
        .with_technical(format!("cabecera desconocida en {}", path.display()))
    })?;

    Ok(ImageProbe {
        format,
        extension: format.extension().to_string(),
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        out.extend_from_slice(&[0u8; 32]);
        out
    }

    fn webp() -> Vec<u8> {
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&64u32.to_le_bytes());
        out.extend_from_slice(b"WEBPVP8 ");
        out.extend_from_slice(&[0u8; 32]);
        out
    }

    #[test]
    fn olfatea_formatos_conocidos() {
        assert_eq!(sniff_image(&png()), Some(DetectedImage::Png));
        assert_eq!(sniff_image(&webp()), Some(DetectedImage::WebP));
        assert_eq!(
            sniff_image(b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01"),
            Some(DetectedImage::Jpeg)
        );
    }

    #[test]
    fn rechaza_lo_que_no_es_imagen() {
        assert_eq!(sniff_image(b"<!DOCTYPE html><html>"), None);
        assert_eq!(sniff_image(b"MZ\x90\0\x03\0\0\0\x04\0\0\0"), None);
        // Un SVG es texto y puede traer scripts: no entra.
        assert_eq!(sniff_image(b"<svg xmlns=\"http://\">"), None);
        // Un WAV empieza con RIFF igual que un WebP, pero no es WEBP.
        assert_eq!(sniff_image(b"RIFF\x24\x00\x00\x00WAVEfmt "), None);
        assert_eq!(sniff_image(b"corto"), None);
    }

    #[test]
    fn valida_una_imagen_real() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("icono.png");
        std::fs::write(&ruta, png()).unwrap();

        let probe = probe_image_file(&ruta).unwrap();
        assert_eq!(probe.format, DetectedImage::Png);
        assert_eq!(probe.extension, "png");
        assert_eq!(probe.size_bytes, png().len() as u64);
    }

    #[test]
    fn la_extension_mentirosa_no_cambia_el_formato_real() {
        let dir = tempfile::tempdir().unwrap();
        // Contenido PNG con nombre .jpg: manda el contenido.
        let ruta = dir.path().join("disfrazada.jpg");
        std::fs::write(&ruta, png()).unwrap();

        assert_eq!(probe_image_file(&ruta).unwrap().extension, "png");
    }

    #[test]
    fn rechaza_vacio_y_no_imagen() {
        let dir = tempfile::tempdir().unwrap();

        let vacia = dir.path().join("vacia.png");
        std::fs::write(&vacia, b"").unwrap();
        assert!(probe_image_file(&vacia).is_err());

        let ejecutable = dir.path().join("virus.png");
        std::fs::write(&ejecutable, b"MZ\x90\x00 esto es un ejecutable").unwrap();
        assert!(probe_image_file(&ejecutable).is_err());
    }
}
