//! Registro y validacion de atajos (§17).
//!
//! Solo los atajos de alcance `Global` se registran en el sistema operativo.
//! Los de alcance `Overlay` los maneja el frontend del overlay mientras tiene
//! el foco: no instalamos hooks de teclado globales.

use std::str::FromStr;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::domain::settings::{ShortcutAction, ShortcutBinding, ShortcutScope, ShortcutSettings};
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{self, ShortcutTriggeredPayload};

/// Atajos globales activos en este momento.
#[derive(Default)]
pub struct ShortcutRegistry {
    active: Mutex<Vec<(ShortcutAction, Shortcut)>>,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accion asociada a un atajo ya registrado.
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        self.active
            .lock()
            .iter()
            .find(|(_, registered)| registered == shortcut)
            .map(|(action, _)| *action)
    }
}

/// Normaliza un acelerador a la forma canonica `Ctrl+Alt+Shift+Super+Tecla`.
///
/// Acepta las variantes habituales (`Control`, `Cmd`, `Win`, `Option`...) para
/// que dos escrituras distintas del mismo atajo se detecten como conflicto.
pub fn normalize_accelerator(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("El atajo no puede estar vacio."));
    }

    let mut control = false;
    let mut alt = false;
    let mut shift = false;
    let mut super_key = false;
    let mut key: Option<String> = None;

    for part in trimmed.split('+') {
        let part = part.trim();
        if part.is_empty() {
            return Err(AppError::validation(
                "El atajo tiene una combinacion incompleta.",
            ));
        }

        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => control = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "super" | "meta" | "cmd" | "command" | "win" | "windows" => super_key = true,
            _ => {
                if key.is_some() {
                    return Err(AppError::validation(
                        "El atajo solo puede tener una tecla ademas de los modificadores.",
                    ));
                }
                key = Some(canonical_key(part));
            }
        }
    }

    let key = key.ok_or_else(|| {
        AppError::validation("El atajo necesita una tecla ademas de los modificadores.")
    })?;

    let mut parts = Vec::new();
    if control {
        parts.push("Ctrl".to_string());
    }
    if alt {
        parts.push("Alt".to_string());
    }
    if shift {
        parts.push("Shift".to_string());
    }
    if super_key {
        parts.push("Super".to_string());
    }
    parts.push(key);

    Ok(parts.join("+"))
}

/// Forma canonica de una tecla suelta.
fn canonical_key(key: &str) -> String {
    let lower = key.to_ascii_lowercase();
    match lower.as_str() {
        "space" | "spacebar" => "Space".to_string(),
        "esc" | "escape" => "Escape".to_string(),
        "pageup" | "prior" => "PageUp".to_string(),
        "pagedown" | "next" => "PageDown".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "del" | "delete" => "Delete".to_string(),
        "ins" | "insert" => "Insert".to_string(),
        "up" => "ArrowUp".to_string(),
        "down" => "ArrowDown".to_string(),
        "left" => "ArrowLeft".to_string(),
        "right" => "ArrowRight".to_string(),
        _ => {
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => lower,
            }
        }
    }
}

/// Valida que el acelerador tenga una forma que el sistema pueda registrar.
/// Un atajo global sin modificadores secuestraria una tecla en todo el sistema.
pub fn validate_accelerator(accelerator: &str, scope: ShortcutScope) -> AppResult<String> {
    let normalized = normalize_accelerator(accelerator)?;

    if scope == ShortcutScope::Global && !normalized.contains('+') {
        return Err(AppError::validation(
            "Un atajo global necesita al menos un modificador (Ctrl, Alt, Shift o Super).",
        ));
    }

    Shortcut::from_str(&normalized).map_err(|error| {
        AppError::new(
            ErrorKind::Shortcut,
            format!("El sistema no reconoce la combinacion \u{201c}{normalized}\u{201d}."),
        )
        .with_technical(error.to_string())
    })?;

    Ok(normalized)
}

/// Conflicto entre dos acciones que comparten el mismo atajo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConflict {
    pub accelerator: String,
    pub actions: Vec<String>,
}

/// Detecta atajos repetidos dentro del mismo alcance.
///
/// Dos acciones pueden compartir combinacion si viven en alcances distintos
/// (una global y otra solo dentro del overlay): no compiten entre si.
pub fn detect_conflicts(bindings: &[ShortcutBinding]) -> Vec<ShortcutConflict> {
    let mut groups: std::collections::BTreeMap<(String, ShortcutScope), Vec<ShortcutAction>> =
        std::collections::BTreeMap::new();

    for binding in bindings {
        let key = normalize_accelerator(&binding.accelerator)
            .unwrap_or_else(|_| binding.accelerator.clone());
        let scope = binding.scope;
        groups.entry((key, scope)).or_default().push(binding.action);
    }

    groups
        .into_iter()
        .filter(|(_, actions)| actions.len() > 1)
        .map(|((accelerator, _), actions)| ShortcutConflict {
            accelerator,
            actions: actions
                .into_iter()
                .map(|action| action.as_str().to_string())
                .collect(),
        })
        .collect()
}

// `ShortcutScope` necesita orden para poder ser clave de un `BTreeMap`.
impl PartialOrd for ShortcutScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ShortcutScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(scope: &ShortcutScope) -> u8 {
            match scope {
                ShortcutScope::Global => 0,
                ShortcutScope::Overlay => 1,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

/// Resultado de intentar registrar los atajos globales.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationReport {
    pub registered: Vec<String>,
    /// Atajos que el sistema rechazo, normalmente porque otra aplicacion los usa.
    pub rejected: Vec<RejectedShortcut>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedShortcut {
    pub action: String,
    pub accelerator: String,
    pub message: String,
}

/// Registra en el sistema todos los atajos globales de la configuracion.
///
/// Libera primero los anteriores. Un atajo rechazado no aborta el resto: se
/// informa y la aplicacion sigue funcionando (§17.6).
pub fn apply(
    app: &AppHandle,
    registry: &ShortcutRegistry,
    settings: &ShortcutSettings,
) -> RegistrationReport {
    let manager = app.global_shortcut();

    if let Err(error) = manager.unregister_all() {
        tracing::warn!(%error, "no se pudieron liberar los atajos anteriores");
    }
    registry.active.lock().clear();

    let mut report = RegistrationReport::default();

    for binding in settings.global_bindings() {
        let normalized = match validate_accelerator(&binding.accelerator, ShortcutScope::Global) {
            Ok(value) => value,
            Err(error) => {
                report.rejected.push(RejectedShortcut {
                    action: binding.action.as_str().to_string(),
                    accelerator: binding.accelerator.clone(),
                    message: error.message.clone(),
                });
                continue;
            }
        };

        let Ok(shortcut) = Shortcut::from_str(&normalized) else {
            continue;
        };

        match manager.register(shortcut) {
            Ok(()) => {
                registry.active.lock().push((binding.action, shortcut));
                report.registered.push(normalized);
            }
            Err(error) => {
                tracing::warn!(
                    action = binding.action.as_str(),
                    accelerator = %normalized,
                    %error,
                    "el sistema rechazo el atajo global"
                );
                report.rejected.push(RejectedShortcut {
                    action: binding.action.as_str().to_string(),
                    accelerator: normalized,
                    message: format!(
                        "Otra aplicacion ya esta usando este atajo para \u{201c}{}\u{201d}.",
                        binding.action.label()
                    ),
                });
            }
        }
    }

    tracing::info!(
        registrados = report.registered.len(),
        rechazados = report.rejected.len(),
        "atajos globales aplicados"
    );
    report
}

/// Notifica al frontend que se disparo un atajo global.
/// El manejo concreto (abrir overlay, detener todo) ocurre en `app.rs`.
pub fn emit_triggered(app: &AppHandle, action: ShortcutAction) {
    events::emit(
        app,
        events::SHORTCUT_TRIGGERED,
        ShortcutTriggeredPayload {
            action: action.as_str().to_string(),
        },
    );
}

/// Solo actuamos al presionar, no al soltar: si no, cada atajo se dispararia dos veces.
pub fn is_press(state: ShortcutState) -> bool {
    state == ShortcutState::Pressed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_variantes_del_mismo_atajo() {
        for entrada in [
            "Ctrl+Alt+Space",
            "control+alt+space",
            "CmdOrCtrl+Option+SpaceBar",
            "  ALT + CTRL + space  ",
        ] {
            assert_eq!(
                normalize_accelerator(entrada).unwrap(),
                "Ctrl+Alt+Space",
                "entrada: {entrada}"
            );
        }
    }

    #[test]
    fn ordena_los_modificadores_de_forma_canonica() {
        assert_eq!(
            normalize_accelerator("Shift+Super+Alt+Ctrl+K").unwrap(),
            "Ctrl+Alt+Shift+Super+K"
        );
        assert_eq!(normalize_accelerator("win+d").unwrap(), "Super+D");
        assert_eq!(normalize_accelerator("PageUp").unwrap(), "PageUp");
        assert_eq!(normalize_accelerator("esc").unwrap(), "Escape");
    }

    #[test]
    fn rechaza_combinaciones_invalidas() {
        assert!(normalize_accelerator("").is_err());
        assert!(normalize_accelerator("   ").is_err());
        assert!(normalize_accelerator("Ctrl+").is_err());
        assert!(normalize_accelerator("Ctrl+Alt").is_err(), "faltan teclas");
        assert!(normalize_accelerator("Ctrl+A+B").is_err(), "dos teclas");
    }

    #[test]
    fn un_atajo_global_exige_modificador() {
        assert!(validate_accelerator("F5", ShortcutScope::Global).is_err());
        assert!(validate_accelerator("Ctrl+Alt+Space", ShortcutScope::Global).is_ok());
        // Dentro del overlay una tecla suelta es perfectamente valida.
        assert!(validate_accelerator("PageUp", ShortcutScope::Overlay).is_ok());
    }

    fn binding(action: ShortcutAction, accelerator: &str, scope: ShortcutScope) -> ShortcutBinding {
        ShortcutBinding {
            action,
            accelerator: accelerator.to_string(),
            scope,
        }
    }

    #[test]
    fn detecta_conflictos_dentro_del_mismo_alcance() {
        let bindings = vec![
            binding(
                ShortcutAction::ToggleOverlay,
                "Ctrl+Alt+Space",
                ShortcutScope::Global,
            ),
            binding(
                ShortcutAction::StopAll,
                "control+alt+SPACE",
                ShortcutScope::Global,
            ),
        ];

        let conflictos = detect_conflicts(&bindings);
        assert_eq!(conflictos.len(), 1);
        assert_eq!(conflictos[0].accelerator, "Ctrl+Alt+Space");
        assert_eq!(conflictos[0].actions.len(), 2);
    }

    #[test]
    fn no_hay_conflicto_entre_alcances_distintos() {
        let bindings = vec![
            binding(
                ShortcutAction::ToggleOverlay,
                "PageUp",
                ShortcutScope::Global,
            ),
            binding(ShortcutAction::PrevPage, "PageUp", ShortcutScope::Overlay),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn la_configuracion_predeterminada_no_tiene_conflictos() {
        let settings = ShortcutSettings::default();
        assert!(detect_conflicts(&settings.bindings).is_empty());

        // Y todos sus aceleradores son validos para su alcance.
        for binding in &settings.bindings {
            validate_accelerator(&binding.accelerator, binding.scope)
                .unwrap_or_else(|error| panic!("{}: {}", binding.accelerator, error.message));
        }
    }
}
