//! Categorias normalizadas y su mapeo determinista (§15).

use serde::{Deserialize, Serialize};

/// Categoria simplificada de la aplicacion. Es metadata secundaria: el buscador
/// sigue siendo el mecanismo principal para encontrar sonidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedCategory {
    Memes,
    Reactions,
    Games,
    Anime,
    MoviesTv,
    Music,
    SoundEffects,
    Voices,
    Sports,
    Other,
    /// Valor por defecto: no obligamos al usuario a catalogar sus audios (§15).
    #[default]
    Uncategorized,
}

impl NormalizedCategory {
    pub const ALL: [NormalizedCategory; 11] = [
        NormalizedCategory::Memes,
        NormalizedCategory::Reactions,
        NormalizedCategory::Games,
        NormalizedCategory::Anime,
        NormalizedCategory::MoviesTv,
        NormalizedCategory::Music,
        NormalizedCategory::SoundEffects,
        NormalizedCategory::Voices,
        NormalizedCategory::Sports,
        NormalizedCategory::Other,
        NormalizedCategory::Uncategorized,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            NormalizedCategory::Memes => "memes",
            NormalizedCategory::Reactions => "reactions",
            NormalizedCategory::Games => "games",
            NormalizedCategory::Anime => "anime",
            NormalizedCategory::MoviesTv => "movies_tv",
            NormalizedCategory::Music => "music",
            NormalizedCategory::SoundEffects => "sound_effects",
            NormalizedCategory::Voices => "voices",
            NormalizedCategory::Sports => "sports",
            NormalizedCategory::Other => "other",
            NormalizedCategory::Uncategorized => "uncategorized",
        }
    }

    pub fn from_str_or_uncategorized(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == value)
            .unwrap_or(NormalizedCategory::Uncategorized)
    }
}

/// Normaliza texto libre para comparar: minusculas, sin acentos, sin signos.
pub fn normalize_text(input: &str) -> String {
    input
        .chars()
        .map(strip_accent)
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mapa minimo de acentos latinos. Evitamos una dependencia de normalizacion
/// Unicode completa: para busqueda y matching de categorias alcanza y sobra.
fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
        'ñ' | 'Ñ' => 'n',
        'ç' | 'Ç' => 'c',
        other => other,
    }
}

/// Reglas deterministas para mapear la categoria de un proveedor.
/// Devuelve `None` cuando no hay evidencia suficiente: no adivinamos agresivamente.
pub fn map_provider_category(raw: &str) -> Option<NormalizedCategory> {
    let normalized = normalize_text(raw);
    if normalized.is_empty() {
        return None;
    }

    // Cada entrada es (subcadena buscada, categoria). El orden importa: las
    // reglas mas especificas van primero.
    const RULES: &[(&str, NormalizedCategory)] = &[
        ("anime", NormalizedCategory::Anime),
        ("manga", NormalizedCategory::Anime),
        ("meme", NormalizedCategory::Memes),
        ("reaction", NormalizedCategory::Reactions),
        ("reaccion", NormalizedCategory::Reactions),
        ("game", NormalizedCategory::Games),
        ("gaming", NormalizedCategory::Games),
        ("videojuego", NormalizedCategory::Games),
        ("movie", NormalizedCategory::MoviesTv),
        ("film", NormalizedCategory::MoviesTv),
        ("cine", NormalizedCategory::MoviesTv),
        ("television", NormalizedCategory::MoviesTv),
        ("tv", NormalizedCategory::MoviesTv),
        ("serie", NormalizedCategory::MoviesTv),
        ("sound effect", NormalizedCategory::SoundEffects),
        ("sfx", NormalizedCategory::SoundEffects),
        ("efecto", NormalizedCategory::SoundEffects),
        ("music", NormalizedCategory::Music),
        ("musica", NormalizedCategory::Music),
        ("song", NormalizedCategory::Music),
        ("voice", NormalizedCategory::Voices),
        ("voz", NormalizedCategory::Voices),
        ("speech", NormalizedCategory::Voices),
        ("sport", NormalizedCategory::Sports),
        ("deporte", NormalizedCategory::Sports),
        ("futbol", NormalizedCategory::Sports),
    ];

    RULES
        .iter()
        .find(|(needle, _)| normalized.contains(needle))
        .map(|(_, category)| *category)
}

/// Inferencia ligera por nombre de archivo para importaciones locales (§10.11).
/// Solo actua ante una coincidencia evidente; si no, `Uncategorized`.
pub fn infer_category_from_filename(filename: &str) -> NormalizedCategory {
    map_provider_category(filename).unwrap_or(NormalizedCategory::Uncategorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapea_categorias_del_prompt() {
        let casos = [
            ("Anime & Manga", NormalizedCategory::Anime),
            ("Games", NormalizedCategory::Games),
            ("Memes", NormalizedCategory::Memes),
            ("Reactions", NormalizedCategory::Reactions),
            ("Movies", NormalizedCategory::MoviesTv),
            ("Television", NormalizedCategory::MoviesTv),
            ("Sound Effects", NormalizedCategory::SoundEffects),
            ("Sports", NormalizedCategory::Sports),
        ];

        for (entrada, esperada) in casos {
            assert_eq!(map_provider_category(entrada), Some(esperada), "{entrada}");
        }
    }

    #[test]
    fn no_adivina_cuando_no_hay_evidencia() {
        assert_eq!(map_provider_category("Xyzzy 42"), None);
        assert_eq!(map_provider_category(""), None);
        assert_eq!(map_provider_category("   "), None);
    }

    #[test]
    fn tolera_acentos_y_mayusculas() {
        assert_eq!(
            map_provider_category("MÚSICA Electrónica"),
            Some(NormalizedCategory::Music)
        );
        assert_eq!(
            map_provider_category("Reacción del público"),
            Some(NormalizedCategory::Reactions)
        );
    }

    #[test]
    fn infiere_desde_nombre_de_archivo_o_deja_sin_categoria() {
        assert_eq!(
            infer_category_from_filename("meme-risa.mp3"),
            NormalizedCategory::Memes
        );
        assert_eq!(
            infer_category_from_filename("grabacion_001.wav"),
            NormalizedCategory::Uncategorized
        );
    }

    #[test]
    fn roundtrip_de_strings() {
        for categoria in NormalizedCategory::ALL {
            assert_eq!(
                NormalizedCategory::from_str_or_uncategorized(categoria.as_str()),
                categoria
            );
        }
        assert_eq!(
            NormalizedCategory::from_str_or_uncategorized("desconocida"),
            NormalizedCategory::Uncategorized
        );
    }

    #[test]
    fn normaliza_texto_para_busqueda() {
        assert_eq!(normalize_text("  Canción  Épica!! "), "cancion epica");
    }
}
