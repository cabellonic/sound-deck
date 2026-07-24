//! Repositorio de paginas de la botonera.

use rusqlite::{params, OptionalExtension};

use crate::domain::{new_id, now_timestamp, PageSummary, SoundPage, MAX_PAGES, SLOTS_PER_PAGE};
use crate::errors::{AppError, AppResult};

use super::{slots, Database};

pub fn count(db: &Database) -> AppResult<i64> {
    let connection = db.lock();
    Ok(connection.query_row("SELECT COUNT(*) FROM pages", [], |row| row.get(0))?)
}

fn validate_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(
            "El nombre de la pagina no puede estar vacio.",
        ));
    }
    if trimmed.chars().count() > 40 {
        return Err(AppError::validation(
            "El nombre de la pagina no puede superar los 40 caracteres.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Crea una pagina con sus nueve slots vacios.
pub fn create(db: &Database, name: &str) -> AppResult<SoundPage> {
    let name = validate_name(name)?;

    let existing = count(db)?;
    if existing as usize >= MAX_PAGES {
        return Err(AppError::validation(format!(
            "Ya alcanzaste el maximo de {MAX_PAGES} paginas. Borra alguna para crear otra."
        )));
    }

    let id = new_id();
    let timestamp = now_timestamp();

    db.transaction(|tx| {
        let next_position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM pages",
            [],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO pages (id, name, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, name, next_position, timestamp],
        )?;

        let mut statement = tx.prepare(
            "INSERT INTO slots (page_id, slot_number, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
        )?;
        for slot_number in 1..=SLOTS_PER_PAGE {
            statement.execute(params![id, slot_number, timestamp])?;
        }
        Ok(())
    })?;

    get(db, &id)?.ok_or_else(|| AppError::database("La pagina recien creada no se encontro."))
}

/// Resumen de todas las paginas, ordenadas por posicion.
pub fn list_summaries(db: &Database) -> AppResult<Vec<PageSummary>> {
    let connection = db.lock();
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, p.position,
                (SELECT COUNT(*) FROM slots s WHERE s.page_id = p.id AND s.sound_id IS NOT NULL)
         FROM pages p ORDER BY p.position ASC",
    )?;
    let summaries = statement
        .query_map([], |row| {
            Ok(PageSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                position: row.get(2)?,
                assigned_slots: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(summaries)
}

/// Una pagina con sus nueve slots resueltos.
pub fn get(db: &Database, page_id: &str) -> AppResult<Option<SoundPage>> {
    let header = {
        let connection = db.lock();
        connection
            .query_row(
                "SELECT id, name, position FROM pages WHERE id = ?1",
                [page_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
    };

    let Some((id, name, position)) = header else {
        return Ok(None);
    };

    Ok(Some(SoundPage {
        slots: slots::list_for_page(db, &id)?,
        id,
        name,
        position,
    }))
}

/// La primera pagina por posicion. Sirve de fallback cuando la pagina activa
/// desaparecio (§39).
pub fn first(db: &Database) -> AppResult<Option<SoundPage>> {
    let id: Option<String> = {
        let connection = db.lock();
        connection
            .query_row(
                "SELECT id FROM pages ORDER BY position ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?
    };

    match id {
        Some(id) => get(db, &id),
        None => Ok(None),
    }
}

pub fn rename(db: &Database, page_id: &str, name: &str) -> AppResult<SoundPage> {
    let name = validate_name(name)?;
    let connection = db.lock();
    let updated = connection.execute(
        "UPDATE pages SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now_timestamp(), page_id],
    )?;
    drop(connection);

    if updated == 0 {
        return Err(AppError::not_found("Esa pagina ya no existe."));
    }
    get(db, page_id)?.ok_or_else(|| AppError::not_found("Esa pagina ya no existe."))
}

/// Cantidad de slots con sonido asignado. El frontend lo usa para decidir si
/// hace falta pedir confirmacion antes de borrar (§8).
pub fn assigned_slot_count(db: &Database, page_id: &str) -> AppResult<i64> {
    let connection = db.lock();
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM slots WHERE page_id = ?1 AND sound_id IS NOT NULL",
        [page_id],
        |row| row.get(0),
    )?)
}

/// Borra una pagina. Nunca borra sonidos de la biblioteca (§8) y siempre deja
/// al menos una pagina en pie.
pub fn delete(db: &Database, page_id: &str) -> AppResult<()> {
    if count(db)? <= 1 {
        return Err(AppError::validation(
            "Debe existir al menos una pagina. Renombrala en lugar de borrarla.",
        ));
    }

    db.transaction(|tx| {
        let removed = tx.execute("DELETE FROM pages WHERE id = ?1", [page_id])?;
        if removed == 0 {
            return Err(AppError::not_found("Esa pagina ya no existe."));
        }
        compact_positions(tx)?;
        Ok(())
    })
}

/// Reordena las paginas segun la lista de ids recibida.
///
/// El indice unico de `position` obliga a un paso intermedio: primero movemos
/// todo a posiciones negativas (libres por construccion) y despues asignamos
/// las definitivas.
pub fn reorder(db: &Database, ordered_ids: &[String]) -> AppResult<Vec<PageSummary>> {
    if ordered_ids.is_empty() {
        return Err(AppError::validation(
            "No se recibio ninguna pagina para reordenar.",
        ));
    }

    db.transaction(|tx| {
        let total: i64 = tx.query_row("SELECT COUNT(*) FROM pages", [], |row| row.get(0))?;
        if total != ordered_ids.len() as i64 {
            return Err(AppError::validation(
                "La lista de paginas para reordenar no coincide con las paginas existentes.",
            ));
        }

        let timestamp = now_timestamp();
        for (index, page_id) in ordered_ids.iter().enumerate() {
            let moved = tx.execute(
                "UPDATE pages SET position = ?1, updated_at = ?2 WHERE id = ?3",
                params![-(index as i64) - 1, timestamp, page_id],
            )?;
            if moved == 0 {
                return Err(AppError::validation(
                    "Una de las paginas indicadas ya no existe.",
                ));
            }
        }

        for index in 0..ordered_ids.len() {
            tx.execute(
                "UPDATE pages SET position = ?1 WHERE position = ?2",
                params![index as i64, -(index as i64) - 1],
            )?;
        }
        Ok(())
    })?;

    list_summaries(db)
}

/// Duplica una pagina con todas sus asignaciones (§8, opcional).
pub fn duplicate(db: &Database, page_id: &str) -> AppResult<SoundPage> {
    let source =
        get(db, page_id)?.ok_or_else(|| AppError::not_found("Esa pagina ya no existe."))?;

    let copy_name = format!("{} (copia)", source.name);
    let truncated: String = copy_name.chars().take(40).collect();
    let new_page = create(db, &truncated)?;

    db.transaction(|tx| {
        let timestamp = now_timestamp();
        for slot in &source.slots {
            tx.execute(
                "UPDATE slots SET sound_id = ?1, custom_label = ?2, custom_volume = ?3,
                        updated_at = ?4
                 WHERE page_id = ?5 AND slot_number = ?6",
                params![
                    slot.sound.as_ref().map(|sound| sound.id.clone()),
                    slot.custom_label,
                    slot.custom_volume,
                    timestamp,
                    new_page.id,
                    slot.slot_number.get(),
                ],
            )?;
        }
        Ok(())
    })?;

    get(db, &new_page.id)?.ok_or_else(|| AppError::database("La pagina duplicada no se encontro."))
}

/// Reasigna posiciones consecutivas desde 0 tras un borrado.
fn compact_positions(tx: &rusqlite::Transaction<'_>) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut statement = tx.prepare("SELECT id FROM pages ORDER BY position ASC")?;
        let collected = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        collected
    };

    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE pages SET position = ?1 WHERE id = ?2",
            params![-(index as i64) - 1, id],
        )?;
    }
    for index in 0..ids.len() {
        tx.execute(
            "UPDATE pages SET position = ?1 WHERE position = ?2",
            params![index as i64, -(index as i64) - 1],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_db;

    #[test]
    fn crear_pagina_genera_nueve_slots_vacios() {
        let db = test_db();
        let pagina = create(&db, "Principal").unwrap();

        assert_eq!(pagina.name, "Principal");
        assert_eq!(pagina.position, 0);
        assert_eq!(pagina.slots.len(), 9);
        assert!(pagina.slots.iter().all(|slot| slot.sound.is_none()));

        let numeros: Vec<u8> = pagina.slots.iter().map(|s| s.slot_number.get()).collect();
        assert_eq!(numeros, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn valida_nombres() {
        let db = test_db();
        assert!(create(&db, "   ").is_err());
        assert!(create(&db, &"x".repeat(41)).is_err());
        assert_eq!(create(&db, "  Discord  ").unwrap().name, "Discord");
    }

    #[test]
    fn respeta_el_maximo_de_paginas() {
        let db = test_db();
        for i in 0..MAX_PAGES {
            create(&db, &format!("Pagina {i}")).unwrap();
        }
        let error = create(&db, "Una mas").unwrap_err();
        assert!(error.message.contains("maximo"));
    }

    #[test]
    fn no_permite_borrar_la_unica_pagina() {
        let db = test_db();
        let pagina = create(&db, "Principal").unwrap();
        let error = delete(&db, &pagina.id).unwrap_err();
        assert!(error.message.contains("al menos una pagina"));
    }

    #[test]
    fn borrar_compacta_las_posiciones() {
        let db = test_db();
        let a = create(&db, "A").unwrap();
        let b = create(&db, "B").unwrap();
        let c = create(&db, "C").unwrap();
        assert_eq!((a.position, b.position, c.position), (0, 1, 2));

        delete(&db, &b.id).unwrap();

        let resumen = list_summaries(&db).unwrap();
        assert_eq!(resumen.len(), 2);
        assert_eq!(resumen[0].position, 0);
        assert_eq!(resumen[1].position, 1);
        assert_eq!(resumen[1].id, c.id);
    }

    #[test]
    fn reordena_invirtiendo_el_orden() {
        let db = test_db();
        let a = create(&db, "A").unwrap();
        let b = create(&db, "B").unwrap();
        let c = create(&db, "C").unwrap();

        let resumen = reorder(&db, &[c.id.clone(), a.id.clone(), b.id.clone()]).unwrap();
        let nombres: Vec<&str> = resumen.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(nombres, vec!["C", "A", "B"]);
    }

    #[test]
    fn reordenar_con_lista_incompleta_falla() {
        let db = test_db();
        let a = create(&db, "A").unwrap();
        create(&db, "B").unwrap();

        assert!(reorder(&db, std::slice::from_ref(&a.id)).is_err());
        assert!(reorder(&db, &[]).is_err());
        // El orden original se conserva tras el rollback.
        assert_eq!(list_summaries(&db).unwrap()[0].name, "A");
    }

    #[test]
    fn renombra_y_falla_si_no_existe() {
        let db = test_db();
        let pagina = create(&db, "Vieja").unwrap();
        assert_eq!(rename(&db, &pagina.id, "Nueva").unwrap().name, "Nueva");
        assert!(rename(&db, "inexistente", "X").is_err());
    }

    #[test]
    fn first_devuelve_la_pagina_de_menor_posicion() {
        let db = test_db();
        assert!(first(&db).unwrap().is_none());
        create(&db, "A").unwrap();
        create(&db, "B").unwrap();
        assert_eq!(first(&db).unwrap().unwrap().name, "A");
    }
}
