//! Medicion de sonoridad y ganancia de normalizacion.
//!
//! Un audio bajado de Internet puede estar quince decibeles por debajo de otro,
//! y hasta ahora la unica salida era acordarse de bajarle el volumen a mano a
//! cada uno. Medimos con EBU R128 (la misma norma que usan las plataformas de
//! streaming) y guardamos el resultado junto al audio; despues, si la opcion
//! esta activada, la reproduccion compensa la diferencia contra un objetivo.
//!
//! Medir es caro: decodifica el archivo entero. Se hace una sola vez, al
//! importar, y el resultado queda en la base.

use std::path::Path;

use ebur128::{EbuR128, Mode};
use rodio::{Decoder, Source};

use crate::errors::{AppError, AppResult, ErrorKind};

/// Sonoridad medida de un audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// Sonoridad integrada en LUFS. Siempre negativa en material normal.
    pub lufs: f32,
    /// Pico maximo de muestra, 0.0..=1.0 (o mas si el archivo viene saturado).
    pub peak: f32,
}

/// Objetivo de sonoridad predeterminado, en LUFS.
///
/// -18 y no los -14 de las plataformas de streaming: una soundboard dispara
/// efectos cortos sobre lo que ya esta sonando, y a -14 tapan la conversacion.
pub const DEFAULT_TARGET_LUFS: f32 = -18.0;

/// Ganancia maxima que puede aplicar la normalizacion.
///
/// Sin tope, un audio casi mudo pediria una ganancia enorme y lo unico que se
/// escucharia seria su ruido de fondo amplificado.
const MAX_GAIN: f32 = 4.0;

/// Sonoridad por debajo de la cual el material se considera silencio y no vale
/// la pena normalizar: `ebur128` devuelve -inf para un archivo mudo.
const SILENCE_LUFS: f32 = -70.0;

/// Mide la sonoridad de un archivo decodificandolo entero.
///
/// Es bloqueante: llamala siempre fuera del hilo principal de Tauri.
pub fn measure_file(path: &Path) -> AppResult<Loudness> {
    let file = std::fs::File::open(path)?;
    let decoder = Decoder::try_from(file).map_err(|error| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "No se pudo decodificar el audio para medir su volumen.",
        )
        .with_technical(format!("{}: {error}", path.display()))
    })?;

    let channels = u32::from(decoder.channels().get());
    let sample_rate = decoder.sample_rate().get();

    let mut meter = EbuR128::new(channels, sample_rate, Mode::I).map_err(|error| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "No se pudo preparar la medicion de volumen de este audio.",
        )
        .with_technical(error.to_string())
    })?;

    // Se alimenta en bloques y no muestra por muestra: `add_frames_f32` tiene
    // un costo fijo por llamada que a una muestra por vez domina todo lo demas.
    let mut block: Vec<f32> = Vec::with_capacity(8192);
    let mut peak = 0.0f32;

    for sample in decoder {
        peak = peak.max(sample.abs());
        block.push(sample);

        if block.len() >= 8192 {
            feed(&mut meter, &block)?;
            block.clear();
        }
    }
    if !block.is_empty() {
        feed(&mut meter, &block)?;
    }

    let lufs = meter.loudness_global().map_err(|error| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "No se pudo calcular el volumen de este audio.",
        )
        .with_technical(error.to_string())
    })? as f32;

    Ok(Loudness {
        lufs: if lufs.is_finite() { lufs } else { SILENCE_LUFS },
        peak,
    })
}

fn feed(meter: &mut EbuR128, samples: &[f32]) -> AppResult<()> {
    meter.add_frames_f32(samples).map_err(|error| {
        AppError::new(
            ErrorKind::InvalidAudio,
            "La medicion de volumen de este audio fallo a mitad de camino.",
        )
        .with_technical(error.to_string())
    })
}

/// Multiplicador que lleva un audio medido al objetivo, sin llegar a saturar.
///
/// `base` es el volumen que ya eligio el usuario: entra en la cuenta porque el
/// tope de saturacion depende de el. Un audio que ya suena al objetivo devuelve
/// 1.0 y no cambia nada.
pub fn normalization_gain(measured: Loudness, target_lufs: f32, base: f32) -> f32 {
    if !measured.lufs.is_finite() || measured.lufs <= SILENCE_LUFS {
        return 1.0;
    }

    let gain = 10f32.powf((target_lufs - measured.lufs) / 20.0);
    let gain = gain.clamp(1.0 / MAX_GAIN, MAX_GAIN);

    // Techo anticlipping: el pico ya amplificado no puede pasarse de la escala.
    // Es lo que evita que normalizar convierta un audio fuerte en distorsion.
    let headroom = if measured.peak > 0.0 && base > 0.0 {
        1.0 / (measured.peak * base)
    } else {
        MAX_GAIN
    };

    gain.min(headroom).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn medido(lufs: f32, peak: f32) -> Loudness {
        Loudness { lufs, peak }
    }

    #[test]
    fn un_audio_en_el_objetivo_no_se_toca() {
        let gain = normalization_gain(medido(-18.0, 0.5), -18.0, 0.35);
        assert!((gain - 1.0).abs() < 0.001, "{gain}");
    }

    #[test]
    fn un_audio_bajo_se_levanta_y_uno_fuerte_se_baja() {
        // 6 dB por debajo del objetivo: hay que duplicar la amplitud.
        let subir = normalization_gain(medido(-24.0, 0.1), -18.0, 0.2);
        assert!((subir - 2.0).abs() < 0.05, "{subir}");

        // 6 dB por encima: a la mitad.
        let bajar = normalization_gain(medido(-12.0, 0.5), -18.0, 0.35);
        assert!((bajar - 0.5).abs() < 0.05, "{bajar}");
    }

    #[test]
    fn nunca_amplifica_hasta_saturar() {
        // Un audio bajo pero con un pico alto: la ganancia se limita para que
        // el pico no se pase de la escala.
        let gain = normalization_gain(medido(-40.0, 1.0), -18.0, 0.8);
        assert!(gain * 1.0 * 0.8 <= 1.0 + f32::EPSILON, "{gain}");
    }

    #[test]
    fn tiene_un_tope_para_no_amplificar_puro_ruido() {
        let gain = normalization_gain(medido(-60.0, 0.001), -18.0, 0.1);
        assert!(gain <= MAX_GAIN, "{gain}");
    }

    #[test]
    fn el_silencio_se_deja_como_esta() {
        assert_eq!(normalization_gain(medido(-70.0, 0.0), -18.0, 0.35), 1.0);
        assert_eq!(
            normalization_gain(medido(f32::NEG_INFINITY, 0.0), -18.0, 0.35),
            1.0
        );
    }

    #[test]
    fn mide_un_tono_generado() {
        // Un tono a media escala tiene que dar una sonoridad razonable y un
        // pico cercano a 0.5, no un valor cualquiera.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tono.wav");
        std::fs::write(&path, tono_wav(1000, 0.5)).unwrap();

        let medida = measure_file(&path).unwrap();
        assert!(medida.peak > 0.45 && medida.peak <= 0.55, "{}", medida.peak);
        assert!(
            medida.lufs > -20.0 && medida.lufs < 0.0,
            "sonoridad inesperada: {}",
            medida.lufs
        );
    }

    #[test]
    fn un_tono_mas_bajo_mide_menos() {
        let dir = tempfile::tempdir().unwrap();
        let fuerte = dir.path().join("fuerte.wav");
        let flojo = dir.path().join("flojo.wav");
        std::fs::write(&fuerte, tono_wav(1000, 0.5)).unwrap();
        std::fs::write(&flojo, tono_wav(1000, 0.05)).unwrap();

        let a = measure_file(&fuerte).unwrap();
        let b = measure_file(&flojo).unwrap();
        assert!(
            a.lufs > b.lufs,
            "{} deberia ser mayor que {}",
            a.lufs,
            b.lufs
        );

        // Y el que mide menos tiene que pedir mas ganancia.
        assert!(
            normalization_gain(b, -18.0, 0.2) > normalization_gain(a, -18.0, 0.2),
            "el audio flojo deberia levantarse mas"
        );
    }

    /// WAV mono de 16 bits con un tono de 440 Hz a la amplitud pedida.
    fn tono_wav(millis: u32, amplitude: f32) -> Vec<u8> {
        let sample_rate = 44100u32;
        let samples = sample_rate * millis / 1000;
        let data_len = samples * 2;

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());

        for index in 0..samples {
            let phase = 2.0 * std::f32::consts::PI * 440.0 * index as f32 / sample_rate as f32;
            let value = (phase.sin() * amplitude * i16::MAX as f32) as i16;
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }
}
