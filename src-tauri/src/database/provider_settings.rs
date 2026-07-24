//! Configuracion por proveedor online.
//!
//! El `settings_json` puede contener secretos (API keys). Nunca se serializa
//! completo hacia el frontend: los comandos devuelven solo un estado y una
//! version enmascarada de la clave (§12).

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::domain::now_timestamp;
use crate::errors::AppResult;

use super::Database;

/// Config cruda de un proveedor tal como vive en la base.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderRecord {
    pub provider_id: String,
    pub enabled: bool,
    pub config: ProviderConfig,
}

pub fn get(db: &Database, provider_id: &str) -> AppResult<Option<ProviderRecord>> {
    let connection = db.lock();
    let row: Option<(i64, String)> = connection
        .query_row(
            "SELECT enabled, settings_json FROM provider_settings WHERE provider_id = ?1",
            [provider_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    Ok(row.map(|(enabled, json)| ProviderRecord {
        provider_id: provider_id.to_string(),
        enabled: enabled != 0,
        config: serde_json::from_str(&json).unwrap_or_default(),
    }))
}

pub fn list(db: &Database) -> AppResult<Vec<ProviderRecord>> {
    let connection = db.lock();
    let mut statement =
        connection.prepare("SELECT provider_id, enabled, settings_json FROM provider_settings")?;
    let records = statement
        .query_map([], |row| {
            let json: String = row.get(2)?;
            Ok(ProviderRecord {
                provider_id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                config: serde_json::from_str(&json).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(records)
}

pub fn set_enabled(db: &Database, provider_id: &str, enabled: bool) -> AppResult<()> {
    let connection = db.lock();
    connection.execute(
        "INSERT INTO provider_settings (provider_id, enabled, settings_json, updated_at)
         VALUES (?1, ?2, '{}', ?3)
         ON CONFLICT(provider_id) DO UPDATE SET enabled = excluded.enabled,
                                                updated_at = excluded.updated_at",
        params![provider_id, i64::from(enabled), now_timestamp()],
    )?;
    Ok(())
}

/// Guarda la API key. Una cadena vacia la borra.
pub fn set_api_key(db: &Database, provider_id: &str, api_key: Option<&str>) -> AppResult<()> {
    let existing = get(db, provider_id)?;
    let mut config = existing
        .as_ref()
        .map(|r| r.config.clone())
        .unwrap_or_default();
    config.api_key = api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let json = serde_json::to_string(&config)?;
    let enabled = existing.map(|r| r.enabled).unwrap_or(false);

    let connection = db.lock();
    connection.execute(
        "INSERT INTO provider_settings (provider_id, enabled, settings_json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(provider_id) DO UPDATE SET settings_json = excluded.settings_json,
                                                updated_at = excluded.updated_at",
        params![provider_id, i64::from(enabled), json, now_timestamp()],
    )?;
    Ok(())
}

/// Version enmascarada de una clave, apta para mostrar en la interfaz.
/// Nunca devolvemos la clave completa despues de guardarla (§12).
pub fn mask_secret(secret: &str) -> String {
    let visible: String = secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if secret.chars().count() <= 4 {
        "••••".to_string()
    } else {
        format!("••••••••{visible}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_db;

    #[test]
    fn guarda_y_recupera_estado_del_proveedor() {
        let db = test_db();
        assert!(get(&db, "freesound").unwrap().is_none());

        set_enabled(&db, "freesound", true).unwrap();
        let record = get(&db, "freesound").unwrap().unwrap();
        assert!(record.enabled);
        assert!(record.config.api_key.is_none());
    }

    #[test]
    fn la_api_key_no_pisa_el_estado_habilitado() {
        let db = test_db();
        set_enabled(&db, "freesound", true).unwrap();
        set_api_key(&db, "freesound", Some("  secreto-largo  ")).unwrap();

        let record = get(&db, "freesound").unwrap().unwrap();
        assert!(record.enabled);
        assert_eq!(record.config.api_key.as_deref(), Some("secreto-largo"));
    }

    #[test]
    fn una_clave_vacia_borra_la_existente() {
        let db = test_db();
        set_api_key(&db, "freesound", Some("abc123")).unwrap();
        set_api_key(&db, "freesound", Some("   ")).unwrap();
        assert!(get(&db, "freesound")
            .unwrap()
            .unwrap()
            .config
            .api_key
            .is_none());
    }

    #[test]
    fn enmascara_secretos() {
        assert_eq!(mask_secret("abcdefghij"), "••••••••ghij");
        assert_eq!(mask_secret("abc"), "••••");
        assert!(!mask_secret("clave-super-secreta").contains("clave"));
    }
}
