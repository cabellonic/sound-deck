//! Modelo de dominio de un sonido de la biblioteca local.

use serde::{Deserialize, Serialize};

use super::category::NormalizedCategory;

/// Origen del sonido. Coincide con la union discriminada de TypeScript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SoundSource {
    LocalImport,
    Provider {
        provider_id: String,
        remote_id: String,
    },
}

impl SoundSource {
    pub fn kind(&self) -> &'static str {
        match self {
            SoundSource::LocalImport => "local_import",
            SoundSource::Provider { .. } => "provider",
        }
    }
}

/// Licencia declarada por el proveedor de origen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundLicense {
    /// Codigo corto y estable (ej. `cc0`, `cc-by-4.0`).
    pub code: String,
    /// Nombre legible para mostrar.
    pub name: String,
    pub url: Option<String>,
}

/// Sonido tal como lo consume el frontend (DTO).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub original_name: Option<String>,
    pub file_extension: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub source: SoundSource,
    pub provider_category: Option<String>,
    pub normalized_category: NormalizedCategory,
    pub tags: Vec<String>,
    pub license: Option<SoundLicense>,
    pub attribution: Option<String>,
    pub source_page_url: Option<String>,
    /// Volumen absoluto propio. `None` = linkeado al volumen general (§18).
    pub custom_volume: Option<f32>,
    /// Ruta absoluta de la imagen administrada, o `None` si no tiene.
    ///
    /// Es la unica ruta de disco que sale al frontend, y sale porque el
    /// protocolo `asset:` de Tauri necesita una ruta para poder mostrarla. El
    /// `assetProtocol.scope` de `tauri.conf.json` la acota a la carpeta de
    /// imagenes, asi que no habilita leer nada mas.
    pub image_path: Option<String>,
    pub play_count: i64,
    pub last_played_at: Option<String>,
    pub created_at: String,
    /// `false` cuando el archivo administrado ya no existe en disco.
    pub file_available: bool,
    /// Cantidad de slots que apuntan a este sonido.
    pub assigned_slot_count: i64,
}

/// Fila completa tal como vive en SQLite. Incluye campos que no salen al frontend
/// (ruta absoluta, hash) para no exponer detalles innecesarios por IPC.
#[derive(Debug, Clone)]
pub struct SoundRecord {
    pub id: String,
    pub name: String,
    pub original_name: Option<String>,
    pub internal_filename: String,
    pub file_path: String,
    pub content_hash: String,
    pub mime_type: Option<String>,
    pub file_extension: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub source_type: String,
    pub provider_id: Option<String>,
    pub remote_id: Option<String>,
    pub source_page_url: Option<String>,
    pub download_url_reference: Option<String>,
    pub provider_category: Option<String>,
    pub normalized_category: String,
    pub license_code: Option<String>,
    pub license_name: Option<String>,
    pub license_url: Option<String>,
    pub attribution: Option<String>,
    pub custom_volume: Option<f32>,
    pub image_path: Option<String>,
    pub play_count: i64,
    pub last_played_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SoundRecord {
    pub fn source(&self) -> SoundSource {
        match (
            self.source_type.as_str(),
            &self.provider_id,
            &self.remote_id,
        ) {
            ("provider", Some(provider_id), Some(remote_id)) => SoundSource::Provider {
                provider_id: provider_id.clone(),
                remote_id: remote_id.clone(),
            },
            _ => SoundSource::LocalImport,
        }
    }

    pub fn license(&self) -> Option<SoundLicense> {
        let code = self.license_code.clone()?;
        Some(SoundLicense {
            name: self.license_name.clone().unwrap_or_else(|| code.clone()),
            url: self.license_url.clone(),
            code,
        })
    }

    /// Convierte a DTO. `tags`, disponibilidad de archivo y uso en slots se
    /// resuelven aparte porque provienen de otras consultas.
    pub fn into_dto(
        self,
        tags: Vec<String>,
        file_available: bool,
        assigned_slot_count: i64,
    ) -> Sound {
        Sound {
            source: self.source(),
            license: self.license(),
            normalized_category: NormalizedCategory::from_str_or_uncategorized(
                &self.normalized_category,
            ),
            id: self.id,
            name: self.name,
            original_name: self.original_name,
            file_extension: self.file_extension,
            file_size_bytes: self.file_size_bytes,
            duration_ms: self.duration_ms,
            provider_category: self.provider_category,
            tags,
            attribution: self.attribution,
            source_page_url: self.source_page_url,
            custom_volume: self.custom_volume,
            image_path: self.image_path,
            play_count: self.play_count,
            last_played_at: self.last_played_at,
            created_at: self.created_at,
            file_available,
            assigned_slot_count,
        }
    }
}

/// Criterio de orden de la busqueda local (§26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SoundSortOrder {
    #[default]
    Relevance,
    Recent,
    MostPlayed,
    Name,
}

/// Filtro rapido de la biblioteca (§9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum LibraryFilter {
    #[default]
    All,
    Recent,
    MostPlayed,
    Unassigned,
    Uncategorized,
    Category {
        category: NormalizedCategory,
    },
    Provider {
        provider_id: String,
    },
}

/// Parametros de `search_local_sounds`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundQuery {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub filter: LibraryFilter,
    #[serde(default)]
    pub sort: SoundSortOrder,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    200
}

impl Default for SoundQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            filter: LibraryFilter::All,
            sort: SoundSortOrder::Relevance,
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serializa_como_union_discriminada() {
        let local = serde_json::to_value(SoundSource::LocalImport).unwrap();
        assert_eq!(local["type"], "local_import");

        let remoto = serde_json::to_value(SoundSource::Provider {
            provider_id: "freesound".into(),
            remote_id: "12345".into(),
        })
        .unwrap();
        assert_eq!(remoto["type"], "provider");
        assert_eq!(remoto["providerId"], "freesound");
        assert_eq!(remoto["remoteId"], "12345");
    }

    #[test]
    fn filtro_por_defecto_es_todos() {
        let query: SoundQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.filter, LibraryFilter::All);
        assert_eq!(query.sort, SoundSortOrder::Relevance);
        assert_eq!(query.limit, 200);
    }
}
