//! Tipos de dominio compartidos por toda la aplicacion.
//!
//! Estos tipos son la fuente de verdad: los equivalentes de TypeScript en
//! `src/types/domain.ts` reflejan exactamente esta forma serializada.

pub mod category;
pub mod page;
pub mod settings;
pub mod sound;

pub use category::NormalizedCategory;
pub use page::{PageSummary, SlotNumber, SoundPage, SoundSlot, MAX_PAGES, SLOTS_PER_PAGE};
pub use settings::{
    AppSettings, AudioSettings, GeneralSettings, PlaybackMode, SettingsPatch, ShortcutAction,
    ShortcutBinding, ShortcutScope, ShortcutSettings,
};
pub use sound::{
    LibraryFilter, Sound, SoundLicense, SoundQuery, SoundRecord, SoundSortOrder, SoundSource,
    SoundUsage,
};

/// Marca de tiempo actual en RFC 3339 UTC. Formato unico para toda la base.
pub fn now_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// Genera un identificador nuevo para entidades persistidas.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
