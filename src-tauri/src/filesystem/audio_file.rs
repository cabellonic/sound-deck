//! Validacion, hash y metadata de archivos de audio (§10, §14).
//!
//! Regla clave: no confiamos en la extension ni en el `Content-Type`. Antes de
//! aceptar un archivo lo olfateamos por contenido y lo abrimos con el decodificador.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use rodio::{Decoder, Source};
use sha2::{Digest, Sha256};

use crate::errors::{AppError, AppResult, ErrorKind};

use super::paths::{is_supported_extension, sanitize_extension};

/// Formato detectado a partir de los bytes iniciales del archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Mp3,
    Wav,
    Ogg,
    Flac,
    Mp4,
}

impl DetectedFormat {
    pub fn extension(self) -> &'static str {
        match self {
            DetectedFormat::Mp3 => "mp3",
            DetectedFormat::Wav => "wav",
            DetectedFormat::Ogg => "ogg",
            DetectedFormat::Flac => "flac",
            DetectedFormat::Mp4 => "m4a",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            DetectedFormat::Mp3 => "audio/mpeg",
            DetectedFormat::Wav => "audio/wav",
            DetectedFormat::Ogg => "audio/ogg",
            DetectedFormat::Flac => "audio/flac",
            DetectedFormat::Mp4 => "audio/mp4",
        }
    }
}

/// Olfatea el formato por los bytes iniciales. Devuelve `None` si no reconoce nada.
pub fn sniff_format(header: &[u8]) -> Option<DetectedFormat> {
    if header.len() < 12 {
        return None;
    }

    if header.starts_with(b"RIFF") && &header[8..12] == b"WAVE" {
        return Some(DetectedFormat::Wav);
    }
    if header.starts_with(b"OggS") {
        return Some(DetectedFormat::Ogg);
    }
    if header.starts_with(b"fLaC") {
        return Some(DetectedFormat::Flac);
    }
    if &header[4..8] == b"ftyp" {
        return Some(DetectedFormat::Mp4);
    }
    // MP3 con tag ID3 o con frame sync directo (0xFF 0xEx/0xFx).
    if header.starts_with(b"ID3") {
        return Some(DetectedFormat::Mp3);
    }
    if header[0] == 0xFF && (header[1] & 0xE0) == 0xE0 {
        return Some(DetectedFormat::Mp3);
    }

    None
}

/// Metadata obtenida al validar un archivo de audio.
#[derive(Debug, Clone)]
pub struct AudioProbe {
    pub format: DetectedFormat,
    /// Extension final que usara el archivo administrado.
    pub extension: String,
    pub mime_type: String,
    pub size_bytes: u64,
    /// `None` cuando el decodificador no puede determinarla (§39).
    pub duration_ms: Option<i64>,
    pub content_hash: String,
}

/// Tamano maximo absoluto aceptado para un archivo importado o descargado.
pub const MAX_AUDIO_BYTES: u64 = 200 * 1024 * 1024;

/// Valida el archivo y extrae su metadata. Es una operacion bloqueante:
/// llamala siempre fuera del hilo principal de Tauri.
pub fn probe_audio_file(path: &Path, max_bytes: u64) -> AppResult<AudioProbe> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        AppError::filesystem("El archivo de audio no existe o no se puede leer.")
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
            ErrorKind::InvalidAudio,
            "El archivo esta vacio.",
        ));
    }

    let limit = max_bytes.min(MAX_AUDIO_BYTES);
    if size_bytes > limit {
        return Err(AppError::new(
            ErrorKind::InvalidAudio,
            format!(
                "El archivo pesa {} y supera el limite de {}.",
                format_bytes(size_bytes),
                format_bytes(limit)
            ),
        ));
    }

    let mut file = File::open(path)?;
    let mut header = [0u8; 16];
    let read = file.read(&mut header)?;
    let format = sniff_format(&header[..read]).ok_or_else(|| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "El archivo no parece ser un audio en un formato soportado (MP3, WAV, OGG, FLAC o M4A).",
        )
        .with_technical(format!("cabecera desconocida en {}", path.display()))
    })?;

    // El decodificador es la prueba definitiva: si no abre, el archivo no sirve.
    let decoded = File::open(path).map_err(AppError::from).and_then(|file| {
        Decoder::try_from(file).map_err(|error| {
            AppError::new(
                ErrorKind::InvalidAudio,
                "El archivo esta danado o usa una codificacion que no se puede decodificar.",
            )
            .with_technical(format!("{}: {error}", path.display()))
        })
    })?;

    let duration_ms = decoded
        .total_duration()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());

    // Preferimos la extension detectada por contenido; solo caemos en la del
    // nombre si el sniffing dio un formato generico compatible con ella.
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(sanitize_extension)
        .filter(|ext| is_supported_extension(ext) && matches_format(format, ext))
        .unwrap_or_else(|| format.extension().to_string());

    Ok(AudioProbe {
        format,
        extension,
        mime_type: format.mime_type().to_string(),
        size_bytes,
        duration_ms,
        content_hash: hash_file(path)?,
    })
}

/// Si la extension del nombre es coherente con el formato detectado.
fn matches_format(format: DetectedFormat, extension: &str) -> bool {
    match format {
        DetectedFormat::Mp3 => extension == "mp3",
        DetectedFormat::Wav => extension == "wav",
        DetectedFormat::Ogg => matches!(extension, "ogg" | "oga"),
        DetectedFormat::Flac => extension == "flac",
        DetectedFormat::Mp4 => matches!(extension, "m4a" | "mp4" | "aac"),
    }
}

/// SHA-256 del contenido, en hexadecimal. Es la clave de deduplicacion (§10).
pub fn hash_file(path: &Path) -> AppResult<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Content-Types que aceptamos de un proveedor. Se valida junto al sniffing:
/// un tipo permitido no alcanza por si solo (§14.8).
pub fn is_acceptable_content_type(content_type: &str) -> bool {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    matches!(
        normalized.as_str(),
        "audio/mpeg"
            | "audio/mp3"
            | "audio/wav"
            | "audio/x-wav"
            | "audio/wave"
            | "audio/ogg"
            | "application/ogg"
            | "audio/vorbis"
            | "audio/flac"
            | "audio/x-flac"
            | "audio/mp4"
            | "audio/aac"
            | "audio/m4a"
            | "audio/x-m4a"
            // Algunos CDNs sirven binarios genericos; el sniffing decide despues.
            | "application/octet-stream"
            | "binary/octet-stream"
    )
}

/// Formatea bytes de manera legible para mensajes de error al usuario.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_de_prueba(millis: u32) -> Vec<u8> {
        // WAV PCM 16 bits mono a 8000 Hz con silencio.
        let sample_rate = 8000u32;
        let samples = sample_rate * millis / 1000;
        let data_len = samples * 2;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend(std::iter::repeat_n(0u8, data_len as usize));
        out
    }

    #[test]
    fn olfatea_formatos_conocidos() {
        assert_eq!(sniff_format(&wav_de_prueba(10)), Some(DetectedFormat::Wav));
        assert_eq!(
            sniff_format(b"OggS\0\0\0\0\0\0\0\0"),
            Some(DetectedFormat::Ogg)
        );
        assert_eq!(
            sniff_format(b"fLaC\0\0\0\x22\0\0\0\0"),
            Some(DetectedFormat::Flac)
        );
        assert_eq!(
            sniff_format(b"ID3\x03\0\0\0\0\0\0\0\0"),
            Some(DetectedFormat::Mp3)
        );
        assert_eq!(
            sniff_format(b"\xff\xfb\x90\x44\0\0\0\0\0\0\0\0"),
            Some(DetectedFormat::Mp3)
        );
        assert_eq!(
            sniff_format(b"\0\0\0\x20ftypM4A "),
            Some(DetectedFormat::Mp4)
        );
    }

    #[test]
    fn rechaza_contenido_que_no_es_audio() {
        assert_eq!(sniff_format(b"<!DOCTYPE html><html>"), None);
        assert_eq!(sniff_format(b"MZ\x90\0\x03\0\0\0\x04\0\0\0"), None);
        assert_eq!(sniff_format(b"corto"), None);
    }

    #[test]
    fn valida_un_wav_real_y_calcula_duracion() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("tono.wav");
        std::fs::write(&ruta, wav_de_prueba(500)).unwrap();

        let probe = probe_audio_file(&ruta, MAX_AUDIO_BYTES).unwrap();
        assert_eq!(probe.format, DetectedFormat::Wav);
        assert_eq!(probe.extension, "wav");
        assert_eq!(probe.mime_type, "audio/wav");
        assert_eq!(probe.content_hash.len(), 64);

        let duracion = probe.duration_ms.expect("el WAV declara su duracion");
        assert!(
            (duracion - 500).abs() <= 5,
            "duracion inesperada: {duracion}"
        );
    }

    #[test]
    fn una_extension_mentirosa_no_cambia_el_formato_real() {
        let dir = tempfile::tempdir().unwrap();
        // Contenido WAV con nombre .mp3: gana el contenido.
        let ruta = dir.path().join("disfrazado.mp3");
        std::fs::write(&ruta, wav_de_prueba(100)).unwrap();

        let probe = probe_audio_file(&ruta, MAX_AUDIO_BYTES).unwrap();
        assert_eq!(probe.format, DetectedFormat::Wav);
        assert_eq!(probe.extension, "wav");
    }

    #[test]
    fn rechaza_archivo_vacio_o_no_audio() {
        let dir = tempfile::tempdir().unwrap();

        let vacio = dir.path().join("vacio.wav");
        std::fs::write(&vacio, b"").unwrap();
        assert!(probe_audio_file(&vacio, MAX_AUDIO_BYTES).is_err());

        let html = dir.path().join("pagina.mp3");
        std::fs::write(&html, b"<!DOCTYPE html><html><body>404</body></html>").unwrap();
        let error = probe_audio_file(&html, MAX_AUDIO_BYTES).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidAudio);
    }

    #[test]
    fn respeta_el_limite_de_tamano() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("grande.wav");
        std::fs::write(&ruta, wav_de_prueba(1000)).unwrap();

        let error = probe_audio_file(&ruta, 100).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidAudio);
        assert!(error.message.contains("limite"));
    }

    #[test]
    fn hash_detecta_contenido_identico() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        let c = dir.path().join("c.wav");
        std::fs::write(&a, wav_de_prueba(100)).unwrap();
        std::fs::write(&b, wav_de_prueba(100)).unwrap();
        std::fs::write(&c, wav_de_prueba(200)).unwrap();

        assert_eq!(hash_file(&a).unwrap(), hash_file(&b).unwrap());
        assert_ne!(hash_file(&a).unwrap(), hash_file(&c).unwrap());
    }

    #[test]
    fn filtra_content_types() {
        assert!(is_acceptable_content_type("audio/mpeg"));
        assert!(is_acceptable_content_type("audio/wav; charset=binary"));
        assert!(is_acceptable_content_type("APPLICATION/OCTET-STREAM"));
        assert!(!is_acceptable_content_type("text/html"));
        assert!(!is_acceptable_content_type("application/json"));
    }

    #[test]
    fn formatea_tamanos() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }
}
