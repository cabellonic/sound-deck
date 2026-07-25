//! Repositorio de slots. Cada pagina tiene siempre nueve filas creadas.

use std::path::Path;

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::settings::clamp_volume;
use crate::domain::{now_timestamp, SlotNumber, SoundSlot, SoundSource};
use crate::errors::{AppError, AppResult};

use super::Database;

/// Consulta que resuelve el slot junto con el sonido asignado, si lo hay.
const SLOT_QUERY: &str = "SELECT
        sl.page_id, sl.slot_number, sl.custom_label, sl.custom_volume,
        s.id, s.name, s.original_name, s.file_extension, s.file_size_bytes, s.duration_ms,
        s.source_type, s.provider_id, s.remote_id, s.source_page_url, s.provider_category,
        s.normalized_category, s.license_code, s.license_name, s.license_url, s.attribution,
        s.custom_volume AS sound_volume, s.image_path, s.play_count, s.last_played_at, s.created_at,
        s.file_path, s.loudness_lufs,
        (SELECT COUNT(*) FROM slots o WHERE o.sound_id = s.id) AS assigned_slots,
        (SELECT COALESCE(GROUP_CONCAT(tag, char(31)), '')
           FROM sound_tags WHERE sound_tags.sound_id = s.id) AS tag_list
    FROM slots sl
    LEFT JOIN sounds s ON s.id = sl.sound_id";

fn row_to_slot(row: &Row<'_>) -> rusqlite::Result<SoundSlot> {
    let slot_number: u8 = row.get("slot_number")?;
    let slot_number = SlotNumber::new(slot_number).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::other(error)),
        )
    })?;

    let sound_id: Option<String> = row.get("id")?;
    let sound = match sound_id {
        None => None,
        Some(sound_id) => {
            let raw_tags: String = row.get("tag_list")?;
            let tags: Vec<String> = if raw_tags.is_empty() {
                Vec::new()
            } else {
                raw_tags.split('\u{1f}').map(str::to_string).collect()
            };

            let file_path: String = row.get("file_path")?;
            let source_type: String = row.get("source_type")?;
            let provider_id: Option<String> = row.get("provider_id")?;
            let remote_id: Option<String> = row.get("remote_id")?;
            let license_code: Option<String> = row.get("license_code")?;
            let license_name: Option<String> = row.get("license_name")?;
            let license_url: Option<String> = row.get("license_url")?;

            Some(crate::domain::Sound {
                id: sound_id,
                name: row.get("name")?,
                original_name: row.get("original_name")?,
                file_extension: row.get("file_extension")?,
                file_size_bytes: row.get("file_size_bytes")?,
                duration_ms: row.get("duration_ms")?,
                source: match (source_type.as_str(), provider_id, remote_id) {
                    ("provider", Some(provider_id), Some(remote_id)) => SoundSource::Provider {
                        provider_id,
                        remote_id,
                    },
                    _ => SoundSource::LocalImport,
                },
                provider_category: row.get("provider_category")?,
                normalized_category: crate::domain::NormalizedCategory::from_str_or_uncategorized(
                    &row.get::<_, String>("normalized_category")?,
                ),
                tags,
                license: license_code.map(|code| crate::domain::SoundLicense {
                    name: license_name.unwrap_or_else(|| code.clone()),
                    url: license_url,
                    code,
                }),
                attribution: row.get("attribution")?,
                source_page_url: row.get("source_page_url")?,
                custom_volume: row.get("sound_volume")?,
                image_path: row.get("image_path")?,
                play_count: row.get("play_count")?,
                last_played_at: row.get("last_played_at")?,
                created_at: row.get("created_at")?,
                file_available: Path::new(&file_path).is_file(),
                loudness_lufs: row.get("loudness_lufs")?,
                assigned_slot_count: row.get("assigned_slots")?,
                // El detalle de "donde esta asignado" solo lo completa la
                // busqueda de la biblioteca, que es el unico lugar que lo
                // muestra. Aca el sonido ya se esta viendo dentro de su boton.
                assigned_slot: None,
            })
        }
    };

    Ok(SoundSlot {
        page_id: row.get("page_id")?,
        slot_number,
        sound,
        custom_label: row.get("custom_label")?,
        custom_volume: row.get("custom_volume")?,
    })
}

/// Los nueve slots de una pagina, en orden.
pub fn list_for_page(db: &Database, page_id: &str) -> AppResult<Vec<SoundSlot>> {
    let connection = db.lock();
    let mut statement = connection.prepare(&format!(
        "{SLOT_QUERY} WHERE sl.page_id = ?1 ORDER BY sl.slot_number ASC"
    ))?;
    let slots = statement
        .query_map([page_id], row_to_slot)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(slots)
}

pub fn get(db: &Database, page_id: &str, slot_number: SlotNumber) -> AppResult<Option<SoundSlot>> {
    let connection = db.lock();
    let slot = connection
        .prepare(&format!(
            "{SLOT_QUERY} WHERE sl.page_id = ?1 AND sl.slot_number = ?2"
        ))?
        .query_row(params![page_id, slot_number.get()], row_to_slot)
        .optional()?;
    Ok(slot)
}

/// Asigna un sonido a un slot. Verifica que ambos existan para no dejar el slot
/// apuntando a un registro inexistente (§7).
pub fn assign(
    db: &Database,
    page_id: &str,
    slot_number: SlotNumber,
    sound_id: &str,
) -> AppResult<SoundSlot> {
    db.transaction(|tx| {
        let sound_exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM sounds WHERE id = ?1",
            [sound_id],
            |row| row.get(0),
        )?;
        if sound_exists == 0 {
            return Err(AppError::not_found(
                "Ese sonido ya no existe en la biblioteca.",
            ));
        }

        let updated = tx.execute(
            "UPDATE slots SET sound_id = ?1, updated_at = ?2
             WHERE page_id = ?3 AND slot_number = ?4",
            params![sound_id, now_timestamp(), page_id, slot_number.get()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("Ese slot no existe."));
        }
        Ok(())
    })?;

    get(db, page_id, slot_number)?.ok_or_else(|| AppError::not_found("Ese slot no existe."))
}

/// Vacia un slot: quita el sonido y su etiqueta o volumen personalizados.
pub fn clear(db: &Database, page_id: &str, slot_number: SlotNumber) -> AppResult<SoundSlot> {
    let connection = db.lock();
    let updated = connection.execute(
        "UPDATE slots SET sound_id = NULL, custom_label = NULL, custom_volume = NULL,
                updated_at = ?1
         WHERE page_id = ?2 AND slot_number = ?3",
        params![now_timestamp(), page_id, slot_number.get()],
    )?;
    drop(connection);

    if updated == 0 {
        return Err(AppError::not_found("Ese slot no existe."));
    }
    get(db, page_id, slot_number)?.ok_or_else(|| AppError::not_found("Ese slot no existe."))
}

/// Intercambia el contenido de dos slots (pueden ser de paginas distintas).
pub fn swap(
    db: &Database,
    from_page: &str,
    from_slot: SlotNumber,
    to_page: &str,
    to_slot: SlotNumber,
) -> AppResult<()> {
    if from_page == to_page && from_slot == to_slot {
        return Ok(());
    }

    db.transaction(|tx| {
        type SlotContent = (Option<String>, Option<String>, Option<f32>);
        let read = |page: &str, slot: u8| -> AppResult<SlotContent> {
            tx.query_row(
                "SELECT sound_id, custom_label, custom_volume FROM slots
                 WHERE page_id = ?1 AND slot_number = ?2",
                params![page, slot],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("Uno de los slots indicados no existe."))
        };

        let origen = read(from_page, from_slot.get())?;
        let destino = read(to_page, to_slot.get())?;
        let timestamp = now_timestamp();

        let write = |page: &str, slot: u8, content: SlotContent| -> AppResult<()> {
            tx.execute(
                "UPDATE slots SET sound_id = ?1, custom_label = ?2, custom_volume = ?3,
                        updated_at = ?4
                 WHERE page_id = ?5 AND slot_number = ?6",
                params![content.0, content.1, content.2, timestamp, page, slot],
            )?;
            Ok(())
        };

        write(from_page, from_slot.get(), destino)?;
        write(to_page, to_slot.get(), origen)?;
        Ok(())
    })
}

/// Etiqueta visible propia del slot. `None` restaura el nombre del sonido.
pub fn set_label(
    db: &Database,
    page_id: &str,
    slot_number: SlotNumber,
    label: Option<&str>,
) -> AppResult<SoundSlot> {
    let cleaned = label
        .map(crate::filesystem::paths::sanitize_display_name)
        .filter(|value| value != "Sin nombre");

    let connection = db.lock();
    connection.execute(
        "UPDATE slots SET custom_label = ?1, updated_at = ?2
         WHERE page_id = ?3 AND slot_number = ?4",
        params![cleaned, now_timestamp(), page_id, slot_number.get()],
    )?;
    drop(connection);

    get(db, page_id, slot_number)?.ok_or_else(|| AppError::not_found("Ese slot no existe."))
}

/// Volumen propio del slot. `None` vuelve a usar el del sonido.
pub fn set_volume(
    db: &Database,
    page_id: &str,
    slot_number: SlotNumber,
    volume: Option<f32>,
) -> AppResult<SoundSlot> {
    let volume = volume.map(clamp_volume);

    let connection = db.lock();
    connection.execute(
        "UPDATE slots SET custom_volume = ?1, updated_at = ?2
         WHERE page_id = ?3 AND slot_number = ?4",
        params![volume, now_timestamp(), page_id, slot_number.get()],
    )?;
    drop(connection);

    get(db, page_id, slot_number)?.ok_or_else(|| AppError::not_found("Ese slot no existe."))
}

/// Quita un sonido de todos los slots que lo usan. Se llama antes de borrarlo.
pub fn clear_all_uses(db: &Database, sound_id: &str) -> AppResult<usize> {
    let connection = db.lock();
    let cleared = connection.execute(
        "UPDATE slots SET sound_id = NULL, updated_at = ?1 WHERE sound_id = ?2",
        params![now_timestamp(), sound_id],
    )?;
    Ok(cleared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sounds::tests::nuevo_sonido;
    use crate::database::{pages, sounds, test_db};

    fn slot(n: u8) -> SlotNumber {
        SlotNumber::new(n).unwrap()
    }

    #[test]
    fn asigna_y_limpia_un_slot() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        let sonido = sounds::insert(&db, nuevo_sonido("Bruh", "h1")).unwrap();

        let asignado = assign(&db, &pagina.id, slot(1), &sonido.id).unwrap();
        assert_eq!(asignado.sound.as_ref().unwrap().id, sonido.id);
        assert_eq!(asignado.display_label(), Some("Bruh"));

        let vaciado = clear(&db, &pagina.id, slot(1)).unwrap();
        assert!(vaciado.sound.is_none());
    }

    #[test]
    fn asignar_un_sonido_inexistente_falla() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        assert!(assign(&db, &pagina.id, slot(1), "no-existe").is_err());
        // El slot queda vacio, sin apuntar a nada roto.
        assert!(get(&db, &pagina.id, slot(1))
            .unwrap()
            .unwrap()
            .sound
            .is_none());
    }

    #[test]
    fn intercambia_slots_de_la_misma_pagina() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        let a = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        let b = sounds::insert(&db, nuevo_sonido("B", "h2")).unwrap();

        assign(&db, &pagina.id, slot(1), &a.id).unwrap();
        assign(&db, &pagina.id, slot(5), &b.id).unwrap();

        swap(&db, &pagina.id, slot(1), &pagina.id, slot(5)).unwrap();

        assert_eq!(
            get(&db, &pagina.id, slot(1))
                .unwrap()
                .unwrap()
                .sound
                .unwrap()
                .name,
            "B"
        );
        assert_eq!(
            get(&db, &pagina.id, slot(5))
                .unwrap()
                .unwrap()
                .sound
                .unwrap()
                .name,
            "A"
        );
    }

    #[test]
    fn mover_a_un_slot_vacio_lo_deja_vacio_en_el_origen() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        let a = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        assign(&db, &pagina.id, slot(1), &a.id).unwrap();

        swap(&db, &pagina.id, slot(1), &pagina.id, slot(9)).unwrap();

        assert!(get(&db, &pagina.id, slot(1))
            .unwrap()
            .unwrap()
            .sound
            .is_none());
        assert_eq!(
            get(&db, &pagina.id, slot(9))
                .unwrap()
                .unwrap()
                .sound
                .unwrap()
                .name,
            "A"
        );
    }

    #[test]
    fn intercambia_entre_paginas_distintas() {
        let db = test_db();
        let uno = pages::create(&db, "Uno").unwrap();
        let dos = pages::create(&db, "Dos").unwrap();
        let a = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        assign(&db, &uno.id, slot(3), &a.id).unwrap();

        swap(&db, &uno.id, slot(3), &dos.id, slot(7)).unwrap();

        assert!(get(&db, &uno.id, slot(3)).unwrap().unwrap().sound.is_none());
        assert_eq!(
            get(&db, &dos.id, slot(7))
                .unwrap()
                .unwrap()
                .sound
                .unwrap()
                .name,
            "A"
        );
    }

    #[test]
    fn borrar_un_sonido_deja_el_slot_vacio_y_no_roto() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        let sonido = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        assign(&db, &pagina.id, slot(2), &sonido.id).unwrap();

        assert_eq!(sounds::usage(&db, &sonido.id).unwrap().len(), 1);

        sounds::delete(&db, &sonido.id).unwrap();

        let slot_actual = get(&db, &pagina.id, slot(2)).unwrap().unwrap();
        assert!(
            slot_actual.sound.is_none(),
            "no deben quedar referencias huerfanas"
        );
    }

    #[test]
    fn etiqueta_y_volumen_personalizados() {
        let db = test_db();
        let pagina = pages::create(&db, "Principal").unwrap();
        let sonido = sounds::insert(&db, nuevo_sonido("Nombre largo", "h1")).unwrap();
        assign(&db, &pagina.id, slot(1), &sonido.id).unwrap();

        let etiquetado = set_label(&db, &pagina.id, slot(1), Some("  Corto  ")).unwrap();
        assert_eq!(etiquetado.display_label(), Some("Corto"));

        let sin_etiqueta = set_label(&db, &pagina.id, slot(1), None).unwrap();
        assert_eq!(sin_etiqueta.display_label(), Some("Nombre largo"));

        let con_volumen = set_volume(&db, &pagina.id, slot(1), Some(5.0)).unwrap();
        assert_eq!(con_volumen.custom_volume, Some(1.0));

        let sin_volumen = set_volume(&db, &pagina.id, slot(1), None).unwrap();
        assert_eq!(sin_volumen.custom_volume, None);
    }

    #[test]
    fn borrar_una_pagina_no_borra_los_sonidos() {
        let db = test_db();
        let uno = pages::create(&db, "Uno").unwrap();
        pages::create(&db, "Dos").unwrap();
        let sonido = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        assign(&db, &uno.id, slot(1), &sonido.id).unwrap();

        pages::delete(&db, &uno.id).unwrap();

        assert!(sounds::find_by_id(&db, &sonido.id).unwrap().is_some());
    }

    #[test]
    fn duplicar_pagina_copia_asignaciones() {
        let db = test_db();
        let original = pages::create(&db, "Original").unwrap();
        let sonido = sounds::insert(&db, nuevo_sonido("A", "h1")).unwrap();
        assign(&db, &original.id, slot(4), &sonido.id).unwrap();
        set_label(&db, &original.id, slot(4), Some("Etiqueta")).unwrap();

        let copia = pages::duplicate(&db, &original.id).unwrap();

        assert_eq!(copia.name, "Original (copia)");
        let copiado = &copia.slots[3];
        assert_eq!(copiado.sound.as_ref().unwrap().id, sonido.id);
        assert_eq!(copiado.custom_label.as_deref(), Some("Etiqueta"));
    }
}
