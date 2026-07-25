//! Repositorio de sonidos de la biblioteca local.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::category::normalize_text;
use crate::domain::sound::{LibraryFilter, SoundSortOrder};
use crate::domain::{
    new_id, now_timestamp, NormalizedCategory, Sound, SoundLicense, SoundQuery, SoundRecord,
    SoundSource, SoundUsage,
};
use crate::errors::{AppError, AppResult};

use super::Database;

/// Datos necesarios para dar de alta un sonido ya validado y copiado a disco.
#[derive(Debug, Clone)]
pub struct NewSound {
    pub name: String,
    pub original_name: Option<String>,
    pub internal_filename: String,
    pub file_path: PathBuf,
    pub content_hash: String,
    pub mime_type: Option<String>,
    pub file_extension: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub source: SoundSource,
    pub source_page_url: Option<String>,
    pub download_url_reference: Option<String>,
    pub provider_category: Option<String>,
    pub normalized_category: NormalizedCategory,
    pub license: Option<SoundLicense>,
    pub attribution: Option<String>,
    pub tags: Vec<String>,
}

const COLUMNS: &str = "id, name, original_name, internal_filename, file_path, content_hash,
     mime_type, file_extension, file_size_bytes, duration_ms, source_type, provider_id,
     remote_id, source_page_url, download_url_reference, provider_category, normalized_category,
     license_code, license_name, license_url, attribution, custom_volume, image_path, play_count,
     last_played_at, created_at, updated_at, loudness_lufs, peak_amplitude";

fn row_to_record(row: &Row<'_>) -> rusqlite::Result<SoundRecord> {
    Ok(SoundRecord {
        id: row.get("id")?,
        name: row.get("name")?,
        original_name: row.get("original_name")?,
        internal_filename: row.get("internal_filename")?,
        file_path: row.get("file_path")?,
        content_hash: row.get("content_hash")?,
        mime_type: row.get("mime_type")?,
        file_extension: row.get("file_extension")?,
        file_size_bytes: row.get("file_size_bytes")?,
        duration_ms: row.get("duration_ms")?,
        source_type: row.get("source_type")?,
        provider_id: row.get("provider_id")?,
        remote_id: row.get("remote_id")?,
        source_page_url: row.get("source_page_url")?,
        download_url_reference: row.get("download_url_reference")?,
        provider_category: row.get("provider_category")?,
        normalized_category: row.get("normalized_category")?,
        license_code: row.get("license_code")?,
        license_name: row.get("license_name")?,
        license_url: row.get("license_url")?,
        attribution: row.get("attribution")?,
        custom_volume: row.get("custom_volume")?,
        image_path: row.get("image_path")?,
        play_count: row.get("play_count")?,
        last_played_at: row.get("last_played_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        loudness_lufs: row.get("loudness_lufs")?,
        peak_amplitude: row.get("peak_amplitude")?,
    })
}

/// Texto normalizado que alimenta la busqueda local (§26).
fn build_search_index(
    name: &str,
    original_name: Option<&str>,
    tags: &[String],
    provider_category: Option<&str>,
    normalized_category: NormalizedCategory,
    provider_id: Option<&str>,
) -> String {
    let mut parts = vec![name.to_string()];
    if let Some(original) = original_name {
        parts.push(original.to_string());
    }
    parts.extend(tags.iter().cloned());
    if let Some(category) = provider_category {
        parts.push(category.to_string());
    }
    parts.push(normalized_category.as_str().replace('_', " "));
    if let Some(provider) = provider_id {
        parts.push(provider.to_string());
    }
    normalize_text(&parts.join(" "))
}

/// Inserta un sonido nuevo. Falla si el hash ya existe (deduplicacion en la base).
pub fn insert(db: &Database, new_sound: NewSound) -> AppResult<Sound> {
    let id = new_id();
    let timestamp = now_timestamp();
    let (source_type, provider_id, remote_id) = match &new_sound.source {
        SoundSource::LocalImport => ("local_import", None, None),
        SoundSource::Provider {
            provider_id,
            remote_id,
        } => (
            "provider",
            Some(provider_id.clone()),
            Some(remote_id.clone()),
        ),
    };

    let mut tags = new_sound.tags.clone();
    tags.sort();
    tags.dedup();

    let search_index = build_search_index(
        &new_sound.name,
        new_sound.original_name.as_deref(),
        &tags,
        new_sound.provider_category.as_deref(),
        new_sound.normalized_category,
        provider_id.as_deref(),
    );

    let file_path = new_sound.file_path.to_string_lossy().to_string();
    let (license_code, license_name, license_url) = match &new_sound.license {
        Some(license) => (
            Some(license.code.clone()),
            Some(license.name.clone()),
            license.url.clone(),
        ),
        None => (None, None, None),
    };

    db.transaction(|tx| {
        tx.execute(
            "INSERT INTO sounds (
                id, name, original_name, internal_filename, file_path, content_hash,
                mime_type, file_extension, file_size_bytes, duration_ms, source_type,
                provider_id, remote_id, source_page_url, download_url_reference,
                provider_category, normalized_category, license_code, license_name, license_url,
                attribution, custom_volume, play_count, search_index, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?21, NULL, 0, ?22, ?23, ?23
            )",
            params![
                id,
                new_sound.name,
                new_sound.original_name,
                new_sound.internal_filename,
                file_path,
                new_sound.content_hash,
                new_sound.mime_type,
                new_sound.file_extension,
                new_sound.file_size_bytes,
                new_sound.duration_ms,
                source_type,
                provider_id,
                remote_id,
                new_sound.source_page_url,
                new_sound.download_url_reference,
                new_sound.provider_category,
                new_sound.normalized_category.as_str(),
                license_code,
                license_name,
                license_url,
                new_sound.attribution,
                search_index,
                timestamp,
            ],
        )?;

        let mut statement =
            tx.prepare("INSERT OR IGNORE INTO sound_tags (sound_id, tag) VALUES (?1, ?2)")?;
        for tag in &tags {
            statement.execute(params![id, tag])?;
        }
        Ok(())
    })?;

    find_by_id(db, &id)?
        .ok_or_else(|| AppError::database("El sonido recien creado no se encontro."))
}

/// Busca un sonido por su identificador.
pub fn find_by_id(db: &Database, id: &str) -> AppResult<Option<Sound>> {
    let connection = db.lock();
    let record = connection
        .prepare(&format!("SELECT {COLUMNS} FROM sounds WHERE id = ?1"))?
        .query_row([id], row_to_record)
        .optional()?;
    drop(connection);

    match record {
        Some(record) => Ok(Some(hydrate(db, record)?)),
        None => Ok(None),
    }
}

/// Devuelve la ruta absoluta en disco de un sonido, sin exponerla al frontend.
pub fn file_path_of(db: &Database, id: &str) -> AppResult<Option<PathBuf>> {
    let connection = db.lock();
    let path: Option<String> = connection
        .query_row("SELECT file_path FROM sounds WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(path.map(PathBuf::from))
}

/// Busca por hash de contenido: la deteccion de duplicados de §10.
pub fn find_by_hash(db: &Database, content_hash: &str) -> AppResult<Option<Sound>> {
    let connection = db.lock();
    let id: Option<String> = connection
        .query_row(
            "SELECT id FROM sounds WHERE content_hash = ?1",
            [content_hash],
            |row| row.get(0),
        )
        .optional()?;
    drop(connection);

    match id {
        Some(id) => find_by_id(db, &id),
        None => Ok(None),
    }
}

/// Busca un sonido ya guardado proveniente de un proveedor concreto.
pub fn find_by_remote(
    db: &Database,
    provider_id: &str,
    remote_id: &str,
) -> AppResult<Option<Sound>> {
    let connection = db.lock();
    let id: Option<String> = connection
        .query_row(
            "SELECT id FROM sounds WHERE provider_id = ?1 AND remote_id = ?2",
            params![provider_id, remote_id],
            |row| row.get(0),
        )
        .optional()?;
    drop(connection);

    match id {
        Some(id) => find_by_id(db, &id),
        None => Ok(None),
    }
}

fn hydrate(db: &Database, record: SoundRecord) -> AppResult<Sound> {
    let tags = tags_of(db, &record.id)?;
    let (assigned, single) = assignment(db, &record.id)?;
    let available = Path::new(&record.file_path).is_file();
    Ok(record.into_dto(tags, available, assigned, single))
}

/// Guarda la sonoridad medida de un audio.
pub fn save_loudness(
    db: &Database,
    sound_id: &str,
    loudness: crate::audio::Loudness,
) -> AppResult<()> {
    let connection = db.lock();
    connection.execute(
        "UPDATE sounds SET loudness_lufs = ?1, peak_amplitude = ?2, updated_at = ?3 WHERE id = ?4",
        params![loudness.lufs, loudness.peak, now_timestamp(), sound_id],
    )?;
    Ok(())
}

/// Sonoridad y pico de un audio, para calcular su ganancia al reproducirlo.
pub fn loudness_of(db: &Database, sound_id: &str) -> AppResult<Option<crate::audio::Loudness>> {
    let connection = db.lock();
    let row: Option<(Option<f32>, Option<f32>)> = connection
        .query_row(
            "SELECT loudness_lufs, peak_amplitude FROM sounds WHERE id = ?1",
            [sound_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    Ok(match row {
        Some((Some(lufs), Some(peak))) => Some(crate::audio::Loudness { lufs, peak }),
        _ => None,
    })
}

/// Audios que todavia no tienen medicion, con su ruta en disco.
pub fn pending_loudness(db: &Database) -> AppResult<Vec<(String, PathBuf)>> {
    let connection = db.lock();
    let mut statement = connection.prepare(
        "SELECT id, file_path FROM sounds
         WHERE loudness_lufs IS NULL OR peak_amplitude IS NULL
         ORDER BY created_at",
    )?;
    let pending = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PathBuf::from(row.get::<_, String>(1)?),
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(pending)
}

pub fn tags_of(db: &Database, sound_id: &str) -> AppResult<Vec<String>> {
    let connection = db.lock();
    let mut statement =
        connection.prepare("SELECT tag FROM sound_tags WHERE sound_id = ?1 ORDER BY tag")?;
    let tags = statement
        .query_map([sound_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

pub fn assigned_slot_count(db: &Database, sound_id: &str) -> AppResult<i64> {
    let connection = db.lock();
    let count = connection.query_row(
        "SELECT COUNT(*) FROM slots WHERE sound_id = ?1",
        [sound_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Lee el unico slot de la fila de `search`, que ya lo trajo en la consulta.
///
/// Solo tiene sentido con `assigned == 1`: con cero no hay nada y con dos o mas
/// las columnas traen una asignacion cualquiera, que sola seria enganosa.
fn single_usage(row: &Row<'_>, assigned: i64) -> rusqlite::Result<Option<SoundUsage>> {
    if assigned != 1 {
        return Ok(None);
    }

    let page_id: Option<String> = row.get("single_page_id")?;
    let page_name: Option<String> = row.get("single_page_name")?;
    let slot_number: Option<u8> = row.get("single_slot_number")?;

    Ok(match (page_id, page_name, slot_number) {
        (Some(page_id), Some(page_name), Some(slot_number)) => Some(SoundUsage {
            page_id,
            page_name,
            slot_number,
        }),
        _ => None,
    })
}

/// Cuantos slots usan el sonido y, cuando es exactamente uno, cual.
///
/// La biblioteca muestra "En Principal - boton 3" en vez de "En 1 boton", que
/// no le dice nada a nadie. Con dos o mas asignaciones no vale la pena traer el
/// detalle: la fila no tiene lugar para enumerarlas.
fn assignment(db: &Database, sound_id: &str) -> AppResult<(i64, Option<SoundUsage>)> {
    let count = assigned_slot_count(db, sound_id)?;
    if count != 1 {
        return Ok((count, None));
    }

    let connection = db.lock();
    let single = connection
        .query_row(
            "SELECT p.id, p.name, sl.slot_number
             FROM slots sl JOIN pages p ON p.id = sl.page_id
             WHERE sl.sound_id = ?1",
            [sound_id],
            |row| {
                Ok(SoundUsage {
                    page_id: row.get(0)?,
                    page_name: row.get(1)?,
                    slot_number: row.get(2)?,
                })
            },
        )
        .optional()?;

    Ok((count, single))
}

/// Busqueda local con filtros y orden (§9, §26).
///
/// Todo el filtrado ocurre en SQL: nunca traemos la biblioteca entera para
/// filtrarla en memoria.
pub fn search(db: &Database, query: &SoundQuery) -> AppResult<Vec<Sound>> {
    let normalized = normalize_text(&query.text);
    let tokens: Vec<String> = normalized
        .split_whitespace()
        .take(8)
        .map(str::to_string)
        .collect();

    let mut conditions: Vec<String> = Vec::new();
    let mut arguments: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    for token in &tokens {
        conditions.push(format!("s.search_index LIKE ?{}", arguments.len() + 1));
        arguments.push(Box::new(format!("%{token}%")));
    }

    match &query.filter {
        LibraryFilter::All | LibraryFilter::Recent | LibraryFilter::MostPlayed => {}
        LibraryFilter::Unassigned => {
            conditions
                .push("NOT EXISTS (SELECT 1 FROM slots WHERE slots.sound_id = s.id)".to_string());
        }
        LibraryFilter::Uncategorized => {
            conditions.push("s.normalized_category = 'uncategorized'".to_string());
        }
        LibraryFilter::Category { category } => {
            conditions.push(format!("s.normalized_category = ?{}", arguments.len() + 1));
            arguments.push(Box::new(category.as_str().to_string()));
        }
        LibraryFilter::Provider { provider_id } => {
            conditions.push(format!("s.provider_id = ?{}", arguments.len() + 1));
            arguments.push(Box::new(provider_id.clone()));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Relevancia: coincidencia exacta, luego prefijo, luego parcial (§26).
    let relevance = if let Some(first) = tokens.first() {
        let exact = arguments.len() + 1;
        let prefix = arguments.len() + 2;
        arguments.push(Box::new(first.clone()));
        arguments.push(Box::new(format!("{first}%")));
        format!(
            "CASE WHEN s.search_index = ?{exact} THEN 0
                  WHEN s.search_index LIKE ?{prefix} THEN 1
                  ELSE 2 END, "
        )
    } else {
        String::new()
    };

    let order = match query.filter {
        LibraryFilter::Recent => "s.created_at DESC".to_string(),
        LibraryFilter::MostPlayed => "s.play_count DESC, s.last_played_at DESC".to_string(),
        _ => match query.sort {
            SoundSortOrder::Relevance => format!("{relevance}s.name COLLATE NOCASE ASC"),
            SoundSortOrder::Recent => "s.created_at DESC".to_string(),
            SoundSortOrder::MostPlayed => "s.play_count DESC, s.created_at DESC".to_string(),
            SoundSortOrder::Name => "s.name COLLATE NOCASE ASC".to_string(),
        },
    };

    let limit_index = arguments.len() + 1;
    let offset_index = arguments.len() + 2;
    arguments.push(Box::new(i64::from(query.limit.clamp(1, 1000))));
    arguments.push(Box::new(i64::from(query.offset)));

    let sql = format!(
        "SELECT {COLUMNS},
                (SELECT COUNT(*) FROM slots WHERE slots.sound_id = s.id) AS assigned_slots,
                -- Donde esta asignado, para nombrarlo cuando esta en un solo
                -- boton. Van como subconsultas y no como JOIN para no duplicar
                -- filas cuando el audio esta en varios; con dos o mas se ignoran.
                -- Las tres resuelven por `idx_slots_sound`.
                (SELECT o.page_id FROM slots o WHERE o.sound_id = s.id LIMIT 1) AS single_page_id,
                (SELECT p.name FROM slots o JOIN pages p ON p.id = o.page_id
                  WHERE o.sound_id = s.id LIMIT 1) AS single_page_name,
                (SELECT o.slot_number FROM slots o WHERE o.sound_id = s.id LIMIT 1)
                    AS single_slot_number,
                (SELECT COALESCE(GROUP_CONCAT(tag, char(31)), '')
                   FROM sound_tags WHERE sound_tags.sound_id = s.id) AS tag_list
         FROM sounds s
         {where_clause}
         ORDER BY {order}
         LIMIT ?{limit_index} OFFSET ?{offset_index}"
    );

    let connection = db.lock();
    let mut statement = connection.prepare(&sql)?;
    let parameters: Vec<&dyn rusqlite::ToSql> =
        arguments.iter().map(|value| value.as_ref() as _).collect();

    let sounds = statement
        .query_map(parameters.as_slice(), |row| {
            let record = row_to_record(row)?;
            let assigned: i64 = row.get("assigned_slots")?;
            let raw_tags: String = row.get("tag_list")?;
            let tags = if raw_tags.is_empty() {
                Vec::new()
            } else {
                let mut tags: Vec<String> = raw_tags.split('\u{1f}').map(str::to_string).collect();
                tags.sort();
                tags
            };
            let available = Path::new(&record.file_path).is_file();
            Ok(record.into_dto(tags, available, assigned, single_usage(row, assigned)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(sounds)
}

/// Cambia el nombre visible y recalcula el indice de busqueda.
pub fn rename(db: &Database, sound_id: &str, name: &str) -> AppResult<Sound> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("El nombre no puede estar vacio."));
    }

    let tags = tags_of(db, sound_id)?;
    let connection = db.lock();
    let (original_name, provider_category, normalized_category, provider_id): (
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT original_name, provider_category, normalized_category, provider_id
             FROM sounds WHERE id = ?1",
            [sound_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?;

    let search_index = build_search_index(
        trimmed,
        original_name.as_deref(),
        &tags,
        provider_category.as_deref(),
        NormalizedCategory::from_str_or_uncategorized(&normalized_category),
        provider_id.as_deref(),
    );

    connection.execute(
        "UPDATE sounds SET name = ?1, search_index = ?2, updated_at = ?3 WHERE id = ?4",
        params![trimmed, search_index, now_timestamp(), sound_id],
    )?;
    drop(connection);

    find_by_id(db, sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))
}

/// Volumen absoluto propio del sonido, limitado a 0.0..=1.0.
/// `None` lo vuelve a linkear al volumen general.
pub fn update_volume(db: &Database, sound_id: &str, volume: Option<f32>) -> AppResult<Sound> {
    let volume = volume.map(crate::domain::settings::clamp_volume);
    let connection = db.lock();
    let updated = connection.execute(
        "UPDATE sounds SET custom_volume = ?1, updated_at = ?2 WHERE id = ?3",
        params![volume, now_timestamp(), sound_id],
    )?;
    drop(connection);

    if updated == 0 {
        return Err(AppError::not_found(
            "Ese sonido ya no existe en la biblioteca.",
        ));
    }
    find_by_id(db, sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))
}

/// Ruta de la imagen actual, para poder borrar el archivo al reemplazarla.
pub fn image_path_of(db: &Database, sound_id: &str) -> AppResult<Option<PathBuf>> {
    let connection = db.lock();
    let path: Option<Option<String>> = connection
        .query_row(
            "SELECT image_path FROM sounds WHERE id = ?1",
            [sound_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(path.flatten().map(PathBuf::from))
}

/// Asocia (o quita, con `None`) la imagen ya copiada a la carpeta administrada.
pub fn set_image(db: &Database, sound_id: &str, image_path: Option<&Path>) -> AppResult<Sound> {
    let stored = image_path.map(|path| path.to_string_lossy().to_string());

    let connection = db.lock();
    let updated = connection.execute(
        "UPDATE sounds SET image_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![stored, now_timestamp(), sound_id],
    )?;
    drop(connection);

    if updated == 0 {
        return Err(AppError::not_found(
            "Ese sonido ya no existe en la biblioteca.",
        ));
    }
    find_by_id(db, sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))
}

/// Reemplaza las etiquetas de un sonido.
pub fn set_tags(db: &Database, sound_id: &str, tags: &[String]) -> AppResult<Sound> {
    let mut cleaned: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty() && tag.chars().count() <= 40)
        .collect();
    cleaned.sort();
    cleaned.dedup();
    cleaned.truncate(32);

    db.transaction(|tx| {
        tx.execute("DELETE FROM sound_tags WHERE sound_id = ?1", [sound_id])?;
        let mut statement =
            tx.prepare("INSERT OR IGNORE INTO sound_tags (sound_id, tag) VALUES (?1, ?2)")?;
        for tag in &cleaned {
            statement.execute(params![sound_id, tag])?;
        }
        Ok(())
    })?;

    // Recalcular el indice reutilizando el nombre actual.
    let name: String = {
        let connection = db.lock();
        connection
            .query_row("SELECT name FROM sounds WHERE id = ?1", [sound_id], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?
    };
    rename(db, sound_id, &name)
}

pub fn usage(db: &Database, sound_id: &str) -> AppResult<Vec<SoundUsage>> {
    let connection = db.lock();
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, sl.slot_number
         FROM slots sl JOIN pages p ON p.id = sl.page_id
         WHERE sl.sound_id = ?1
         ORDER BY p.position, sl.slot_number",
    )?;
    let usages = statement
        .query_map([sound_id], |row| {
            Ok(SoundUsage {
                page_id: row.get(0)?,
                page_name: row.get(1)?,
                slot_number: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(usages)
}

/// Borra el registro y devuelve la ruta del archivo que hay que eliminar.
/// Las asignaciones a slots se limpian por `ON DELETE SET NULL`.
pub fn delete(db: &Database, sound_id: &str) -> AppResult<PathBuf> {
    let path = file_path_of(db, sound_id)?
        .ok_or_else(|| AppError::not_found("Ese sonido ya no existe en la biblioteca."))?;

    db.transaction(|tx| {
        tx.execute("DELETE FROM sounds WHERE id = ?1", [sound_id])?;
        Ok(())
    })?;

    Ok(path)
}

/// Registra una reproduccion, para el filtro "mas usados".
pub fn record_play(db: &Database, sound_id: &str) -> AppResult<()> {
    let connection = db.lock();
    connection.execute(
        "UPDATE sounds SET play_count = play_count + 1, last_played_at = ?1 WHERE id = ?2",
        params![now_timestamp(), sound_id],
    )?;
    Ok(())
}

/// Categorias y proveedores presentes en la biblioteca. Alimentan los filtros
/// automaticos, que solo se muestran si existen sonidos correspondientes (§9).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFacets {
    pub total: i64,
    pub unassigned: i64,
    pub categories: Vec<FacetCount>,
    pub providers: Vec<FacetCount>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

pub fn facets(db: &Database) -> AppResult<LibraryFacets> {
    let connection = db.lock();

    let total = connection.query_row("SELECT COUNT(*) FROM sounds", [], |row| row.get(0))?;
    let unassigned = connection.query_row(
        "SELECT COUNT(*) FROM sounds s WHERE NOT EXISTS
            (SELECT 1 FROM slots WHERE slots.sound_id = s.id)",
        [],
        |row| row.get(0),
    )?;

    let mut categories_statement = connection.prepare(
        "SELECT normalized_category, COUNT(*) FROM sounds
         GROUP BY normalized_category ORDER BY COUNT(*) DESC",
    )?;
    let categories = categories_statement
        .query_map([], |row| {
            Ok(FacetCount {
                value: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut providers_statement = connection.prepare(
        "SELECT provider_id, COUNT(*) FROM sounds
         WHERE provider_id IS NOT NULL GROUP BY provider_id ORDER BY COUNT(*) DESC",
    )?;
    let providers = providers_statement
        .query_map([], |row| {
            Ok(FacetCount {
                value: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(LibraryFacets {
        total,
        unassigned,
        categories,
        providers,
    })
}

/// Sonidos cuyo archivo administrado desaparecio del disco (§20 Biblioteca).
pub fn find_missing_files(db: &Database) -> AppResult<Vec<Sound>> {
    let all = search(
        db,
        &SoundQuery {
            limit: 1000,
            ..SoundQuery::default()
        },
    )?;
    Ok(all
        .into_iter()
        .filter(|sound| !sound.file_available)
        .collect())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::database::test_db;

    pub(crate) fn nuevo_sonido(nombre: &str, hash: &str) -> NewSound {
        NewSound {
            name: nombre.to_string(),
            original_name: Some(format!("{nombre}.mp3")),
            internal_filename: format!("{}.mp3", new_id()),
            file_path: PathBuf::from(format!("/tmp/{hash}.mp3")),
            content_hash: hash.to_string(),
            mime_type: Some("audio/mpeg".into()),
            file_extension: Some("mp3".into()),
            file_size_bytes: Some(1024),
            duration_ms: Some(1500),
            source: SoundSource::LocalImport,
            source_page_url: None,
            download_url_reference: None,
            provider_category: None,
            normalized_category: NormalizedCategory::Uncategorized,
            license: None,
            attribution: None,
            tags: vec![],
        }
    }

    #[test]
    fn inserta_y_recupera_un_sonido() {
        let db = test_db();
        let creado = insert(&db, nuevo_sonido("Bruh", "hash-1")).unwrap();

        assert_eq!(creado.name, "Bruh");
        // Un audio nuevo nace linkeado al volumen general y sin imagen.
        assert_eq!(creado.custom_volume, None);
        assert_eq!(creado.image_path, None);
        assert_eq!(creado.play_count, 0);
        assert_eq!(creado.assigned_slot_count, 0);
        // El archivo no existe realmente en el test: debe reportarse como faltante.
        assert!(!creado.file_available);

        let recuperado = find_by_id(&db, &creado.id).unwrap().unwrap();
        assert_eq!(recuperado.id, creado.id);
    }

    /// La biblioteca nombra el boton cuando el audio esta asignado a uno solo;
    /// con dos o mas solo muestra la cantidad.
    #[test]
    fn informa_el_unico_boton_donde_esta_asignado() {
        use crate::database::{pages, slots};
        use crate::domain::SlotNumber;

        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("Bruh", "hash-asignacion")).unwrap();
        let pagina = pages::create(&db, "Memes").unwrap();

        let buscar = || {
            search(&db, &SoundQuery::default())
                .unwrap()
                .into_iter()
                .find(|encontrado| encontrado.id == sonido.id)
                .unwrap()
        };

        // Sin asignar no hay boton que nombrar.
        let libre = buscar();
        assert_eq!(libre.assigned_slot_count, 0);
        assert_eq!(libre.assigned_slot, None);

        slots::assign(&db, &pagina.id, SlotNumber::new(3).unwrap(), &sonido.id).unwrap();

        let en_uno = buscar();
        assert_eq!(en_uno.assigned_slot_count, 1);
        let asignacion = en_uno.assigned_slot.unwrap();
        assert_eq!(asignacion.page_name, "Memes");
        assert_eq!(asignacion.slot_number, 3);
        assert_eq!(asignacion.page_id, pagina.id);
        // `find_by_id` resuelve lo mismo por su propio camino.
        assert_eq!(
            find_by_id(&db, &sonido.id)
                .unwrap()
                .unwrap()
                .assigned_slot
                .unwrap()
                .slot_number,
            3
        );

        slots::assign(&db, &pagina.id, SlotNumber::new(7).unwrap(), &sonido.id).unwrap();

        // Con dos botones el detalle sobra: nombrar uno solo seria enganoso.
        let en_dos = buscar();
        assert_eq!(en_dos.assigned_slot_count, 2);
        assert_eq!(en_dos.assigned_slot, None);
        assert_eq!(
            find_by_id(&db, &sonido.id).unwrap().unwrap().assigned_slot,
            None
        );
    }

    #[test]
    fn el_hash_duplicado_es_rechazado_por_la_base() {
        let db = test_db();
        insert(&db, nuevo_sonido("A", "mismo-hash")).unwrap();
        let repetido = insert(&db, nuevo_sonido("B", "mismo-hash"));
        assert!(repetido.is_err());

        let existente = find_by_hash(&db, "mismo-hash").unwrap().unwrap();
        assert_eq!(existente.name, "A");
    }

    #[test]
    fn busca_por_tokens_ignorando_acentos_y_mayusculas() {
        let db = test_db();
        insert(&db, nuevo_sonido("Canción Épica", "h1")).unwrap();
        insert(&db, nuevo_sonido("Risa malvada", "h2")).unwrap();

        let buscar = |texto: &str| {
            search(
                &db,
                &SoundQuery {
                    text: texto.to_string(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        assert_eq!(buscar("cancion").len(), 1);
        assert_eq!(buscar("EPICA").len(), 1);
        assert_eq!(buscar("cancion epica").len(), 1);
        assert_eq!(buscar("risa").len(), 1);
        assert_eq!(buscar("").len(), 2);
        assert_eq!(buscar("inexistente").len(), 0);
    }

    #[test]
    fn ordena_por_relevancia_exacta_antes_que_parcial() {
        let db = test_db();
        insert(&db, nuevo_sonido("bonk extendido remix", "h1")).unwrap();
        insert(&db, nuevo_sonido("bonk", "h2")).unwrap();

        let resultados = search(
            &db,
            &SoundQuery {
                text: "bonk".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(resultados.len(), 2);
        assert_eq!(resultados[0].name, "bonk");
    }

    #[test]
    fn renombra_y_actualiza_el_indice_de_busqueda() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("Viejo", "h1")).unwrap();

        rename(&db, &sonido.id, "  Nombre Nuevo  ").unwrap();
        let resultados = search(
            &db,
            &SoundQuery {
                text: "nuevo".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].name, "Nombre Nuevo");

        assert!(rename(&db, &sonido.id, "   ").is_err());
    }

    #[test]
    fn el_volumen_se_limita_al_rango_valido() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("A", "h1")).unwrap();

        let volumen = |v: Option<f32>| update_volume(&db, &sonido.id, v).unwrap().custom_volume;

        assert_eq!(volumen(Some(2.0)), Some(1.0));
        assert_eq!(volumen(Some(-3.0)), Some(0.0));
        assert_eq!(volumen(Some(0.5)), Some(0.5));
        // `None` lo vuelve a linkear al volumen general.
        assert_eq!(volumen(None), None);
    }

    #[test]
    fn la_imagen_se_asocia_y_se_quita() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("A", "h1")).unwrap();
        let imagen = PathBuf::from("/tmp/images/abc.png");

        let con_imagen = set_image(&db, &sonido.id, Some(&imagen)).unwrap();
        assert_eq!(
            con_imagen.image_path.as_deref(),
            Some(imagen.to_string_lossy().as_ref())
        );
        assert_eq!(image_path_of(&db, &sonido.id).unwrap(), Some(imagen));

        let sin_imagen = set_image(&db, &sonido.id, None).unwrap();
        assert_eq!(sin_imagen.image_path, None);
        assert_eq!(image_path_of(&db, &sonido.id).unwrap(), None);

        assert!(set_image(&db, "no-existe", None).is_err());
    }

    #[test]
    fn las_etiquetas_se_normalizan_y_deduplican() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("A", "h1")).unwrap();

        let actualizado = set_tags(
            &db,
            &sonido.id,
            &["  Meme ".into(), "meme".into(), "RISA".into(), "".into()],
        )
        .unwrap();

        assert_eq!(
            actualizado.tags,
            vec!["meme".to_string(), "risa".to_string()]
        );

        // Las etiquetas entran al indice de busqueda.
        let resultados = search(
            &db,
            &SoundQuery {
                text: "risa".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(resultados.len(), 1);
    }

    #[test]
    fn cuenta_reproducciones() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("A", "h1")).unwrap();

        record_play(&db, &sonido.id).unwrap();
        record_play(&db, &sonido.id).unwrap();

        let actualizado = find_by_id(&db, &sonido.id).unwrap().unwrap();
        assert_eq!(actualizado.play_count, 2);
        assert!(actualizado.last_played_at.is_some());
    }

    #[test]
    fn facetas_reflejan_lo_que_existe() {
        let db = test_db();
        insert(&db, nuevo_sonido("A", "h1")).unwrap();
        let mut con_proveedor = nuevo_sonido("B", "h2");
        con_proveedor.source = SoundSource::Provider {
            provider_id: "freesound".into(),
            remote_id: "42".into(),
        };
        con_proveedor.normalized_category = NormalizedCategory::Memes;
        insert(&db, con_proveedor).unwrap();

        let facetas = facets(&db).unwrap();
        assert_eq!(facetas.total, 2);
        assert_eq!(facetas.unassigned, 2);
        assert_eq!(facetas.providers.len(), 1);
        assert_eq!(facetas.providers[0].value, "freesound");
        assert!(facetas
            .categories
            .iter()
            .any(|facet| facet.value == "memes" && facet.count == 1));
    }

    #[test]
    fn borrar_devuelve_la_ruta_del_archivo() {
        let db = test_db();
        let sonido = insert(&db, nuevo_sonido("A", "h1")).unwrap();

        let ruta = delete(&db, &sonido.id).unwrap();
        assert!(ruta.to_string_lossy().ends_with("h1.mp3"));
        assert!(find_by_id(&db, &sonido.id).unwrap().is_none());
        assert!(delete(&db, &sonido.id).is_err());
    }
}
