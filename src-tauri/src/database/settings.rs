//! Persistencia de la configuracion: una fila JSON por seccion.

use rusqlite::{params, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

use crate::domain::now_timestamp;
use crate::domain::settings::{
    AppSettings, AudioSettings, GeneralSettings, LibrarySettings, SettingsPatch, ShortcutSettings,
};
use crate::errors::AppResult;

use super::Database;

const SECTION_GENERAL: &str = "general";
const SECTION_AUDIO: &str = "audio";
const SECTION_SHORTCUTS: &str = "shortcuts";
const SECTION_LIBRARY: &str = "library";

/// Lee una seccion. Si falta o esta corrupta, devuelve los valores por defecto:
/// una configuracion invalida no debe impedir que la aplicacion arranque (§39).
fn read_section<T: DeserializeOwned + Default>(db: &Database, section: &str) -> AppResult<T> {
    let connection = db.lock();
    let raw: Option<String> = connection
        .query_row(
            "SELECT value_json FROM settings WHERE section = ?1",
            [section],
            |row| row.get(0),
        )
        .optional()?;
    drop(connection);

    let Some(raw) = raw else {
        return Ok(T::default());
    };

    match serde_json::from_str(&raw) {
        Ok(value) => Ok(value),
        Err(error) => {
            tracing::warn!(
                section,
                %error,
                "la seccion de configuracion no pudo leerse; se usan los valores predeterminados"
            );
            Ok(T::default())
        }
    }
}

fn write_section<T: Serialize>(db: &Database, section: &str, value: &T) -> AppResult<()> {
    let json = serde_json::to_string(value)?;
    let connection = db.lock();
    connection.execute(
        "INSERT INTO settings (section, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(section) DO UPDATE SET value_json = excluded.value_json,
                                            updated_at = excluded.updated_at",
        params![section, json, now_timestamp()],
    )?;
    Ok(())
}

pub fn load(db: &Database) -> AppResult<AppSettings> {
    Ok(AppSettings {
        general: read_section(db, SECTION_GENERAL)?,
        audio: read_section(db, SECTION_AUDIO)?,
        shortcuts: read_section(db, SECTION_SHORTCUTS)?,
        library: read_section(db, SECTION_LIBRARY)?,
    })
}

pub fn save(db: &Database, settings: &AppSettings) -> AppResult<()> {
    write_section(db, SECTION_GENERAL, &settings.general)?;
    write_section(db, SECTION_AUDIO, &settings.audio)?;
    write_section(db, SECTION_SHORTCUTS, &settings.shortcuts)?;
    write_section(db, SECTION_LIBRARY, &settings.library)?;
    Ok(())
}

/// Aplica un parche parcial y devuelve la configuracion resultante.
pub fn apply_patch(db: &Database, patch: SettingsPatch) -> AppResult<AppSettings> {
    let mut settings = load(db)?;

    if let Some(general) = patch.general {
        settings.general = general;
        write_section(db, SECTION_GENERAL, &settings.general)?;
    }
    if let Some(mut audio) = patch.audio {
        audio.master_volume = crate::domain::settings::clamp_volume(audio.master_volume);
        audio.preview_volume = crate::domain::settings::clamp_volume(audio.preview_volume);
        settings.audio = audio;
        write_section(db, SECTION_AUDIO, &settings.audio)?;
    }
    if let Some(shortcuts) = patch.shortcuts {
        settings.shortcuts = shortcuts;
        write_section(db, SECTION_SHORTCUTS, &settings.shortcuts)?;
    }
    if let Some(library) = patch.library {
        settings.library = library;
        write_section(db, SECTION_LIBRARY, &settings.library)?;
    }

    Ok(settings)
}

/// Restablece toda la configuracion a los valores predeterminados.
pub fn reset(db: &Database) -> AppResult<AppSettings> {
    let defaults = AppSettings::default();
    save(db, &defaults)?;
    Ok(defaults)
}

/// Guarda el dispositivo de salida elegido sin tocar el resto de la seccion.
pub fn save_output_device(
    db: &Database,
    device_id: Option<String>,
    device_name: Option<String>,
) -> AppResult<AudioSettings> {
    let mut audio: AudioSettings = read_section(db, SECTION_AUDIO)?;
    audio.output_device_id = device_id;
    audio.output_device_name = device_name;
    write_section(db, SECTION_AUDIO, &audio)?;
    Ok(audio)
}

/// Guarda la seccion general sin tocar el resto de la configuracion.
pub fn save_general(db: &Database, general: &GeneralSettings) -> AppResult<()> {
    write_section(db, SECTION_GENERAL, general)
}

/// Recuerda la ultima pagina activa (§43 "recordar ultima pagina").
pub fn save_last_page(db: &Database, page_id: &str) -> AppResult<()> {
    let mut general: GeneralSettings = read_section(db, SECTION_GENERAL)?;
    if !general.remember_last_page {
        return Ok(());
    }
    general.last_page_id = Some(page_id.to_string());
    write_section(db, SECTION_GENERAL, &general)
}

pub fn save_shortcuts(db: &Database, shortcuts: &ShortcutSettings) -> AppResult<()> {
    write_section(db, SECTION_SHORTCUTS, shortcuts)
}

pub fn save_library(db: &Database, library: &LibrarySettings) -> AppResult<()> {
    write_section(db, SECTION_LIBRARY, library)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_db;
    use crate::domain::settings::{PlaybackMode, ThemePreference};

    #[test]
    fn devuelve_defaults_cuando_no_hay_nada_guardado() {
        let db = test_db();
        assert_eq!(load(&db).unwrap(), AppSettings::default());
    }

    #[test]
    fn guarda_y_recupera() {
        let db = test_db();
        let mut settings = AppSettings::default();
        settings.audio.master_volume = 0.8;
        settings.audio.playback_mode = PlaybackMode::Overlap;
        settings.general.theme = ThemePreference::Light;

        save(&db, &settings).unwrap();

        let recuperado = load(&db).unwrap();
        assert_eq!(recuperado.audio.master_volume, 0.8);
        assert_eq!(recuperado.audio.playback_mode, PlaybackMode::Overlap);
        assert_eq!(recuperado.general.theme, ThemePreference::Light);
    }

    #[test]
    fn el_parche_solo_toca_las_secciones_presentes() {
        let db = test_db();
        let mut inicial = AppSettings::default();
        inicial.general.theme = ThemePreference::Dark;
        save(&db, &inicial).unwrap();

        let patch: SettingsPatch =
            serde_json::from_str(r#"{"audio":{"masterVolume":0.9}}"#).unwrap();
        let resultado = apply_patch(&db, patch).unwrap();

        assert_eq!(resultado.audio.master_volume, 0.9);
        assert_eq!(resultado.general.theme, ThemePreference::Dark);
    }

    #[test]
    fn el_parche_limita_los_volumenes() {
        let db = test_db();
        let patch: SettingsPatch =
            serde_json::from_str(r#"{"audio":{"masterVolume":5,"previewVolume":-2}}"#).unwrap();
        let resultado = apply_patch(&db, patch).unwrap();

        assert_eq!(resultado.audio.master_volume, 1.0);
        assert_eq!(resultado.audio.preview_volume, 0.0);
    }

    #[test]
    fn una_seccion_corrupta_cae_a_los_valores_predeterminados() {
        let db = test_db();
        {
            let connection = db.lock();
            connection
                .execute(
                    "INSERT INTO settings (section, value_json, updated_at)
                     VALUES ('audio', '{{no es json', 'now')",
                    [],
                )
                .unwrap();
        }

        let settings = load(&db).unwrap();
        assert_eq!(settings.audio, AudioSettings::default());
    }

    #[test]
    fn recuerda_la_ultima_pagina_solo_si_esta_activada_la_opcion() {
        let db = test_db();
        save_last_page(&db, "pagina-1").unwrap();
        assert_eq!(
            load(&db).unwrap().general.last_page_id.as_deref(),
            Some("pagina-1")
        );

        let mut settings = load(&db).unwrap();
        settings.general.remember_last_page = false;
        settings.general.last_page_id = None;
        save(&db, &settings).unwrap();

        save_last_page(&db, "pagina-2").unwrap();
        assert_eq!(load(&db).unwrap().general.last_page_id, None);
    }

    #[test]
    fn guarda_el_dispositivo_sin_pisar_el_resto_del_audio() {
        let db = test_db();
        let mut settings = AppSettings::default();
        settings.audio.master_volume = 0.7;
        save(&db, &settings).unwrap();

        let audio =
            save_output_device(&db, Some("device-id".into()), Some("CABLE Input".into())).unwrap();

        assert_eq!(audio.output_device_id.as_deref(), Some("device-id"));
        assert_eq!(audio.master_volume, 0.7);
    }

    #[test]
    fn reset_vuelve_a_los_valores_del_prompt() {
        let db = test_db();
        let mut settings = AppSettings::default();
        settings.audio.master_volume = 1.0;
        save(&db, &settings).unwrap();

        let restablecido = reset(&db).unwrap();
        assert_eq!(restablecido.audio.master_volume, 0.35);
        assert_eq!(load(&db).unwrap().audio.master_volume, 0.35);
    }
}
