//! Persistencia con SQLite.
//!
//! ## Por que rusqlite y no sqlx
//!
//! La base es local, de un solo proceso y con consultas cortas. `rusqlite` con
//! la feature `bundled` compila SQLite dentro del binario, lo que elimina la
//! dependencia de una libreria del sistema (importante para el instalador de
//! Windows) y evita arrastrar un runtime asincronico solo para leer metadata.
//! Las operaciones pesadas (decodificar, descargar, hashear) ya corren fuera del
//! hilo principal, y el acceso a la base se serializa con un mutex explicito.

pub mod migrations;
pub mod pages;
pub mod provider_settings;
pub mod settings;
pub mod slots;
pub mod sounds;

use std::path::Path;
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};
use rusqlite::Connection;

use crate::errors::{AppError, AppResult};

/// Handle compartido a la base. Clonar es barato: comparte el mismo mutex.
#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    /// Abre (o crea) la base en disco y aplica las migraciones pendientes.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path).map_err(|error| {
            AppError::database(
                "No se pudo abrir la base de datos. Puede estar en uso por otra instancia.",
            )
            .with_technical(format!("{}: {error}", path.display()))
            .not_recoverable()
        })?;

        Self::from_connection(connection)
    }

    /// Base en memoria, para tests.
    pub fn open_in_memory() -> AppResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> AppResult<Self> {
        connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        migrations::run(&mut connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Acceso exclusivo a la conexion. El guard es corto por diseno: nunca
    /// hagas E/S de red ni decodificacion de audio mientras lo tengas tomado.
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock()
    }

    /// Ejecuta un bloque dentro de una transaccion. Si el bloque devuelve `Err`,
    /// se hace rollback automatico al soltar la transaccion.
    pub fn transaction<T>(
        &self,
        body: impl FnOnce(&rusqlite::Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;
        let value = body(&transaction)?;
        transaction.commit()?;
        Ok(value)
    }

    /// Crea la base con el estado inicial esperado en el primer arranque (§32):
    /// una unica pagina llamada "Principal".
    pub fn ensure_initial_state(&self) -> AppResult<()> {
        let existing = pages::count(self)?;
        if existing == 0 {
            pages::create(self, "Principal")?;
            tracing::info!("primer arranque: se creo la pagina inicial");
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn test_db() -> Database {
    Database::open_in_memory().expect("base en memoria")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abre_en_disco_y_persiste_entre_conexiones() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("sub").join("database.sqlite");

        {
            let db = Database::open(&ruta).unwrap();
            db.ensure_initial_state().unwrap();
            assert_eq!(pages::count(&db).unwrap(), 1);
        }

        let db = Database::open(&ruta).unwrap();
        assert_eq!(pages::count(&db).unwrap(), 1);
    }

    #[test]
    fn ensure_initial_state_es_idempotente() {
        let db = test_db();
        db.ensure_initial_state().unwrap();
        db.ensure_initial_state().unwrap();
        assert_eq!(pages::count(&db).unwrap(), 1);
    }

    #[test]
    fn las_claves_foraneas_estan_activas() {
        let db = test_db();
        let connection = db.lock();
        let enabled: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
    }

    #[test]
    fn la_transaccion_hace_rollback_ante_error() {
        let db = test_db();
        db.ensure_initial_state().unwrap();

        let resultado: AppResult<()> = db.transaction(|tx| {
            tx.execute(
                "INSERT INTO pages (id, name, position, created_at, updated_at)
                 VALUES ('x', 'Temporal', 99, 'now', 'now')",
                [],
            )?;
            Err(AppError::validation("fallo simulado"))
        });

        assert!(resultado.is_err());
        assert_eq!(pages::count(&db).unwrap(), 1);
    }
}
