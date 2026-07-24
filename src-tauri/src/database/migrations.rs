//! Migraciones de esquema.
//!
//! rusqlite no trae un runner propio, asi que usamos uno explicito y minimo:
//! una tabla `schema_migrations` y un array ordenado de scripts embebidos.
//! Cada migracion se aplica dentro de una transaccion; si falla, no queda a medias.

use rusqlite::Connection;

use crate::errors::{AppError, AppResult};

/// Una migracion embebida en el binario.
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
    /// La migracion reconstruye una tabla referenciada por claves foraneas.
    ///
    /// SQLite obliga a apagar `foreign_keys` para eso, y el pragma es un no-op
    /// dentro de una transaccion: hay que apagarlo antes de abrirla. Sin esto,
    /// el `DROP TABLE sounds` dispararia el `ON DELETE SET NULL` de
    /// `slots.sound_id` y vaciaria la botonera del usuario.
    rebuilds_tables: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("../../migrations/001_initial.sql"),
        rebuilds_tables: false,
    },
    Migration {
        version: 2,
        name: "absolute_sound_volume",
        sql: include_str!("../../migrations/002_absolute_sound_volume.sql"),
        rebuilds_tables: true,
    },
    Migration {
        version: 3,
        name: "sound_image",
        sql: include_str!("../../migrations/003_sound_image.sql"),
        rebuilds_tables: false,
    },
];

/// Aplica todas las migraciones pendientes. Devuelve la version final.
pub fn run(connection: &mut Connection) -> AppResult<i64> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    let current: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        tracing::info!(
            version = migration.version,
            name = migration.name,
            "aplicando migracion"
        );
        apply(connection, migration)?;
    }

    let final_version = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);
    Ok(final_version.max(current))
}

/// Aplica una migracion siguiendo el procedimiento que documenta SQLite para
/// los cambios de esquema que `ALTER TABLE` no cubre: apagar las claves
/// foraneas, hacer todo el trabajo dentro de una transaccion y verificar la
/// integridad referencial antes de confirmar.
fn apply(connection: &mut Connection, migration: &Migration) -> AppResult<()> {
    if migration.rebuilds_tables {
        connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    }

    let result = apply_in_transaction(connection, migration);

    if migration.rebuilds_tables {
        // Se reactivan pase lo que pase: el resto de la aplicacion da por
        // sentado que las claves foraneas estan activas.
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    }

    result
}

fn apply_in_transaction(connection: &mut Connection, migration: &Migration) -> AppResult<()> {
    let failed = |error: rusqlite::Error| {
        AppError::database(format!(
            "La migracion {} ({}) de la base de datos fallo. La aplicacion no puede continuar.",
            migration.version, migration.name
        ))
        .with_technical(error.to_string())
        .not_recoverable()
    };

    let transaction = connection.transaction()?;
    transaction.execute_batch(migration.sql).map_err(failed)?;

    if migration.rebuilds_tables {
        // Con las claves apagadas nadie valido nada mientras corria el script.
        // Este es el unico punto donde podemos detectar una reconstruccion que
        // dejo referencias colgadas, y todavia estamos a tiempo de abortar.
        let violations: i64 = transaction
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(failed)?;

        if violations > 0 {
            return Err(AppError::database(format!(
                "La migracion {} ({}) dejo la base en un estado inconsistente y se revirtio.",
                migration.version, migration.name
            ))
            .with_technical(format!("{violations} violaciones de clave foranea"))
            .not_recoverable());
        }
    }

    transaction.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            migration.version,
            migration.name,
            crate::domain::now_timestamp()
        ],
    )?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Version mas alta declarada, para que agregar una migracion no obligue a
    /// tocar cada assert de este modulo.
    fn ultima_version() -> i64 {
        MIGRATIONS.last().unwrap().version
    }

    #[test]
    fn aplica_migraciones_y_es_idempotente() {
        let mut connection = Connection::open_in_memory().unwrap();

        let version = run(&mut connection).unwrap();
        assert_eq!(version, ultima_version());

        // Correr de nuevo no debe fallar ni reaplicar nada.
        let version_again = run(&mut connection).unwrap();
        assert_eq!(version_again, ultima_version());

        let applied: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied, MIGRATIONS.len() as i64);
    }

    /// Aplica solo hasta cierta version, para poder poblar la base con el
    /// esquema viejo y despues migrarla de verdad.
    fn run_hasta(connection: &mut Connection, version: i64) {
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (
                    version    INTEGER PRIMARY KEY,
                    name       TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                );",
            )
            .unwrap();
        for migration in MIGRATIONS.iter().filter(|m| m.version <= version) {
            apply(connection, migration).unwrap();
        }
    }

    /// La conversion de volumen tiene que preservar lo que el usuario escuchaba:
    /// un multiplicador de 0.5 sobre un general de 0.30 sonaba al 15%, y despues
    /// de migrar tiene que seguir sonando al 15%, ahora como valor absoluto.
    #[test]
    fn el_volumen_multiplicador_se_convierte_en_absoluto() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        run_hasta(&mut connection, 1);

        connection
            .execute_batch(
                r#"
                INSERT INTO settings (section, value_json, updated_at)
                VALUES ('audio', '{"masterVolume":0.30}', 'now');

                INSERT INTO sounds (id, name, internal_filename, file_path, content_hash,
                                    source_type, custom_volume, created_at, updated_at)
                VALUES ('linkeado', 'A', 'a.mp3', '/tmp/a.mp3', 'ha', 'local_import', 1.0, 'now', 'now'),
                       ('propio',   'B', 'b.mp3', '/tmp/b.mp3', 'hb', 'local_import', 0.5, 'now', 'now'),
                       ('mudo',     'C', 'c.mp3', '/tmp/c.mp3', 'hc', 'local_import', 0.0, 'now', 'now');

                INSERT INTO pages (id, name, position, created_at, updated_at)
                VALUES ('p1', 'Principal', 0, 'now', 'now');

                INSERT INTO slots (page_id, slot_number, sound_id, created_at, updated_at)
                VALUES ('p1', 1, 'linkeado', 'now', 'now'),
                       ('p1', 2, 'propio', 'now', 'now');
                "#,
            )
            .unwrap();

        run(&mut connection).unwrap();

        let volumen = |id: &str| -> Option<f64> {
            connection
                .query_row(
                    "SELECT custom_volume FROM sounds WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )
                .unwrap()
        };

        // 1.0 era "no toques nada": pasa a seguir el volumen general.
        assert_eq!(volumen("linkeado"), None);
        // 0.5 sobre un general de 0.30 sonaba al 15%.
        assert_eq!(volumen("propio"), Some(0.15));
        // El silencio explicito se conserva, no se confunde con "linkeado".
        assert_eq!(volumen("mudo"), Some(0.0));
    }

    /// La reconstruccion de `sounds` no debe llevarse puestas las asignaciones:
    /// con las claves foraneas activas, el DROP TABLE dispararia el
    /// `ON DELETE SET NULL` de `slots` y el usuario perderia la botonera.
    #[test]
    fn reconstruir_sounds_conserva_las_asignaciones_de_la_botonera() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        run_hasta(&mut connection, 1);

        connection
            .execute_batch(
                "INSERT INTO sounds (id, name, internal_filename, file_path, content_hash,
                                     source_type, custom_volume, created_at, updated_at)
                 VALUES ('s1', 'A', 'a.mp3', '/tmp/a.mp3', 'ha', 'local_import', 1.0, 'now', 'now');

                 INSERT INTO sound_tags (sound_id, tag) VALUES ('s1', 'meme');

                 INSERT INTO pages (id, name, position, created_at, updated_at)
                 VALUES ('p1', 'Principal', 0, 'now', 'now');

                 INSERT INTO slots (page_id, slot_number, sound_id, custom_label, created_at, updated_at)
                 VALUES ('p1', 3, 's1', 'Etiqueta', 'now', 'now');",
            )
            .unwrap();

        run(&mut connection).unwrap();

        let asignado: Option<String> = connection
            .query_row(
                "SELECT sound_id FROM slots WHERE page_id = 'p1' AND slot_number = 3",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(asignado.as_deref(), Some("s1"));

        // Las etiquetas cuelgan de `sounds` con ON DELETE CASCADE: tampoco se
        // tienen que haber ido.
        let etiquetas: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sound_tags WHERE sound_id = 's1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(etiquetas, 1);

        // Y las claves foraneas quedaron activas para el resto de la sesion.
        let activas: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(activas, 1);
    }

    #[test]
    fn crea_todas_las_tablas_del_modelo() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();

        for table in [
            "sounds",
            "sound_tags",
            "pages",
            "slots",
            "settings",
            "provider_settings",
            "schema_migrations",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "falta la tabla {table}");
        }
    }

    #[test]
    fn el_check_de_slot_number_rechaza_valores_fuera_de_rango() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO pages (id, name, position, created_at, updated_at)
                 VALUES ('p1', 'Principal', 0, 'now', 'now')",
                [],
            )
            .unwrap();

        let valido = connection.execute(
            "INSERT INTO slots (page_id, slot_number, created_at, updated_at)
             VALUES ('p1', 9, 'now', 'now')",
            [],
        );
        assert!(valido.is_ok());

        let invalido = connection.execute(
            "INSERT INTO slots (page_id, slot_number, created_at, updated_at)
             VALUES ('p1', 10, 'now', 'now')",
            [],
        );
        assert!(invalido.is_err());
    }
}
