//! Configuracion de la aplicacion y sus valores predeterminados (§20, §43).
//!
//! Se persiste como una fila JSON por seccion en la tabla `settings`. Cada campo
//! usa `#[serde(default)]` para que agregar opciones nuevas no rompa instalaciones
//! existentes.

use serde::{Deserialize, Serialize};

/// Modo de reproduccion cuando llega un sonido nuevo (§18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    /// El sonido nuevo detiene al anterior.
    #[default]
    Interrupt,
    /// Se permiten varios sonidos simultaneos.
    Overlap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub start_with_system: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub show_notifications: bool,
    pub overlay_on_active_monitor: bool,
    pub close_overlay_after_play: bool,
    pub close_overlay_on_blur: bool,
    pub remember_last_page: bool,
    pub theme: ThemePreference,
    /// Codigo ISO. Preparado para i18n; por ahora solo `es`.
    pub language: String,
    pub onboarding_completed: bool,
    /// Ultima pagina activa, para `remember_last_page`.
    pub last_page_id: Option<String>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            start_with_system: false,
            minimize_to_tray: true,
            close_to_tray: true,
            show_notifications: true,
            overlay_on_active_monitor: true,
            close_overlay_after_play: true,
            close_overlay_on_blur: true,
            remember_last_page: true,
            theme: ThemePreference::System,
            language: "es".to_string(),
            onboarding_completed: false,
            last_page_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    /// Identificador estable del dispositivo (cpal `DeviceId`), cuando existe.
    pub output_device_id: Option<String>,
    /// Nombre legible, usado como fallback si el id ya no resuelve.
    pub output_device_name: Option<String>,
    pub master_volume: f32,
    pub preview_volume: f32,
    pub playback_mode: PlaybackMode,
    /// Si volver a disparar el mismo sonido lo reinicia desde el principio.
    pub restart_same_sound: bool,
    /// Limite de tamano para descargas, en bytes.
    pub max_download_bytes: u64,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            output_device_id: None,
            output_device_name: None,
            master_volume: 0.35,
            preview_volume: 0.20,
            playback_mode: PlaybackMode::Interrupt,
            restart_same_sound: true,
            max_download_bytes: 25 * 1024 * 1024,
        }
    }
}

/// Accion a la que se puede asociar un atajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    ToggleOverlay,
    StopAll,
    PrevPage,
    NextPage,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 4] = [
        ShortcutAction::ToggleOverlay,
        ShortcutAction::StopAll,
        ShortcutAction::PrevPage,
        ShortcutAction::NextPage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "toggle_overlay",
            ShortcutAction::StopAll => "stop_all",
            ShortcutAction::PrevPage => "prev_page",
            ShortcutAction::NextPage => "next_page",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "Abrir/cerrar overlay",
            ShortcutAction::StopAll => "Detener todos los sonidos",
            ShortcutAction::PrevPage => "Pagina anterior",
            ShortcutAction::NextPage => "Pagina siguiente",
        }
    }
}

/// Alcance del atajo: registrado en el sistema o solo activo dentro del overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutScope {
    Global,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    /// Acelerador en formato Tauri, por ejemplo `Ctrl+Alt+Space`.
    pub accelerator: String,
    pub scope: ShortcutScope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutSettings {
    pub bindings: Vec<ShortcutBinding>,
    /// Reproduccion global de 1-9 sin overlay. Desactivada por defecto (§43).
    pub global_slot_playback: bool,
    /// Repetir al mantener una tecla presionada dentro del overlay.
    pub allow_key_repeat: bool,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            bindings: default_bindings(),
            global_slot_playback: false,
            allow_key_repeat: false,
        }
    }
}

/// Atajos predeterminados sugeridos por §17.
pub fn default_bindings() -> Vec<ShortcutBinding> {
    vec![
        ShortcutBinding {
            action: ShortcutAction::ToggleOverlay,
            accelerator: "Ctrl+Alt+Space".to_string(),
            scope: ShortcutScope::Global,
        },
        ShortcutBinding {
            action: ShortcutAction::StopAll,
            accelerator: "Ctrl+Alt+0".to_string(),
            scope: ShortcutScope::Global,
        },
        ShortcutBinding {
            action: ShortcutAction::PrevPage,
            accelerator: "PageUp".to_string(),
            scope: ShortcutScope::Overlay,
        },
        ShortcutBinding {
            action: ShortcutAction::NextPage,
            accelerator: "PageDown".to_string(),
            scope: ShortcutScope::Overlay,
        },
    ]
}

impl ShortcutSettings {
    pub fn accelerator_for(&self, action: ShortcutAction) -> Option<&str> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.accelerator.as_str())
    }

    pub fn global_bindings(&self) -> impl Iterator<Item = &ShortcutBinding> {
        self.bindings
            .iter()
            .filter(|binding| binding.scope == ShortcutScope::Global)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibrarySettings {
    /// Nivel de log activo (`error`, `warn`, `info`, `debug`, `trace`).
    pub log_level: String,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
        }
    }
}

/// Configuracion completa. Es el DTO que consume la pantalla de ajustes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub audio: AudioSettings,
    pub shortcuts: ShortcutSettings,
    pub library: LibrarySettings,
}

/// Parche parcial de configuracion. Solo se aplican las secciones presentes.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub general: Option<GeneralSettings>,
    pub audio: Option<AudioSettings>,
    pub shortcuts: Option<ShortcutSettings>,
    pub library: Option<LibrarySettings>,
}

/// Limita un volumen al rango seguro 0.0..=1.0 (§18). NaN cae a 0.
pub fn clamp_volume(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// Volumen efectivo de una reproduccion: `slot ?? sound ?? master`.
///
/// Los volumenes propios son absolutos, no multiplicadores. `None` en los dos
/// niveles significa "linkeado": manda el volumen general. Un valor deslinquea
/// ese audio o ese boton, que a partir de ahi suena siempre igual aunque el
/// general cambie, y puede incluso quedar mas fuerte que el general.
pub fn effective_volume(master: f32, sound: Option<f32>, slot: Option<f32>) -> f32 {
    clamp_volume(slot.or(sound).unwrap_or(master))
}

/// Volumen efectivo de una previsualizacion: `sound ?? preview`.
///
/// Un audio deslinqueado suena a su volumen tambien al previsualizarlo: la
/// previsualizacion existe para escuchar como va a sonar de verdad.
pub fn effective_preview_volume(preview: f32, sound: Option<f32>) -> f32 {
    clamp_volume(sound.unwrap_or(preview))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valores_predeterminados_del_prompt() {
        let settings = AppSettings::default();
        assert_eq!(settings.audio.master_volume, 0.35);
        assert_eq!(settings.audio.preview_volume, 0.20);
        assert_eq!(settings.audio.playback_mode, PlaybackMode::Interrupt);
        assert!(settings.audio.restart_same_sound);
        assert!(settings.general.close_overlay_after_play);
        assert!(settings.general.close_overlay_on_blur);
        assert!(settings.general.minimize_to_tray);
        assert!(settings.general.remember_last_page);
        assert!(!settings.shortcuts.global_slot_playback);
        assert_eq!(
            settings
                .shortcuts
                .accelerator_for(ShortcutAction::ToggleOverlay),
            Some("Ctrl+Alt+Space")
        );
        assert_eq!(
            settings.shortcuts.accelerator_for(ShortcutAction::StopAll),
            Some("Ctrl+Alt+0")
        );
    }

    #[test]
    fn sin_volumen_propio_manda_el_general() {
        assert_eq!(effective_volume(0.35, None, None), 0.35);
        assert_eq!(effective_volume(0.80, None, None), 0.80);
    }

    #[test]
    fn el_volumen_propio_del_audio_es_absoluto_e_ignora_el_general() {
        // El caso que motivo el cambio: un audio que revienta se baja a 15% y
        // se queda ahi, sin importar donde este el general.
        assert_eq!(effective_volume(0.30, Some(0.15), None), 0.15);
        assert_eq!(effective_volume(0.90, Some(0.15), None), 0.15);
        // Y puede quedar mas fuerte que el general, para levantar un audio bajo.
        assert_eq!(effective_volume(0.30, Some(0.60), None), 0.60);
    }

    #[test]
    fn el_volumen_del_boton_gana_sobre_el_del_audio() {
        assert_eq!(effective_volume(0.35, Some(0.80), Some(0.10)), 0.10);
        // Y un boton sin volumen propio cae al del audio, no al general.
        assert_eq!(effective_volume(0.35, Some(0.80), None), 0.80);
    }

    #[test]
    fn el_volumen_efectivo_se_limita_al_rango_valido() {
        assert_eq!(effective_volume(2.0, None, None), 1.0);
        assert_eq!(effective_volume(0.5, Some(-1.0), None), 0.0);
        assert_eq!(effective_volume(0.5, None, Some(3.0)), 1.0);
        assert_eq!(effective_volume(f32::NAN, None, None), 0.0);
    }

    #[test]
    fn la_preview_respeta_el_volumen_propio_del_audio() {
        // Sin volumen propio manda el de previsualizacion.
        assert_eq!(effective_preview_volume(0.20, None), 0.20);
        // Con volumen propio se escucha como va a sonar de verdad.
        assert_eq!(effective_preview_volume(0.20, Some(0.15)), 0.15);
    }

    #[test]
    fn solo_los_atajos_globales_se_registran_en_el_sistema() {
        let settings = ShortcutSettings::default();
        let globales: Vec<_> = settings.global_bindings().map(|b| b.action).collect();
        assert_eq!(
            globales,
            vec![ShortcutAction::ToggleOverlay, ShortcutAction::StopAll]
        );
    }

    #[test]
    fn configuracion_vacia_se_completa_con_defaults() {
        let settings: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, AppSettings::default());

        let parcial: GeneralSettings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(parcial.theme, ThemePreference::Dark);
        // El resto conserva los valores predeterminados.
        assert!(parcial.minimize_to_tray);
    }
}
