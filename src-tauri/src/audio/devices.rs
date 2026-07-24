//! Enumeracion y resolucion de dispositivos de salida (§18).
//!
//! Usamos `cpal` a traves del re-export de rodio para garantizar que ambas
//! bibliotecas hablen de los mismos tipos.

use std::str::FromStr;

use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::cpal::{self, Device, DeviceId};
use serde::Serialize;

use crate::errors::{AppError, AppResult, ErrorKind};

/// Dispositivo de salida tal como lo ve la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    /// Identificador estable `host:device`. `None` si el backend no lo expone.
    pub id: Option<String>,
    pub name: String,
    pub is_default: bool,
}

/// Como se resolvio el dispositivo pedido. Permite avisar al usuario de forma
/// discreta cuando no se pudo respetar su eleccion (§18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceResolution {
    /// Se encontro por identificador estable.
    ExactId,
    /// El id ya no existe, pero coincidio el nombre legible.
    ByName,
    /// No se encontro nada: se usa el predeterminado del sistema.
    FallbackDefault,
    /// No habia preferencia guardada.
    SystemDefault,
}

impl DeviceResolution {
    /// Mensaje para el usuario, o `None` cuando no hace falta molestarlo.
    pub fn notice(self, requested: Option<&str>) -> Option<String> {
        match self {
            DeviceResolution::ExactId | DeviceResolution::SystemDefault => None,
            DeviceResolution::ByName => Some(
                "El dispositivo guardado cambio de identificador. Se reconecto por su nombre."
                    .to_string(),
            ),
            DeviceResolution::FallbackDefault => Some(format!(
                "No se encontro el dispositivo {}. Se usara el predeterminado del sistema.",
                requested
                    .map(|name| format!("\u{201c}{name}\u{201d}"))
                    .unwrap_or_else(|| "guardado".to_string())
            )),
        }
    }
}

fn describe(device: &Device, default_id: Option<&DeviceId>) -> AudioDeviceInfo {
    let id = device.id().ok();
    let name = device
        .description()
        .map(|description| description.name().to_string())
        .unwrap_or_else(|_| "Dispositivo desconocido".to_string());

    AudioDeviceInfo {
        is_default: match (&id, default_id) {
            (Some(id), Some(default_id)) => id == default_id,
            _ => false,
        },
        id: id.map(|id| id.to_string()),
        name,
    }
}

/// Lista los dispositivos de salida disponibles, con el predeterminado marcado.
pub fn list_output_devices() -> AppResult<Vec<AudioDeviceInfo>> {
    let host = cpal::default_host();
    let default_id = host
        .default_output_device()
        .and_then(|device| device.id().ok());

    let devices = host.output_devices().map_err(|error| {
        AppError::new(
            ErrorKind::AudioDevice,
            "No se pudieron listar los dispositivos de salida del sistema.",
        )
        .with_technical(error.to_string())
    })?;

    let mut listed: Vec<AudioDeviceInfo> = devices
        .map(|device| describe(&device, default_id.as_ref()))
        .collect();

    // El predeterminado primero; el resto alfabetico. Nombres duplicados son
    // posibles (§39), por eso el id es lo que persistimos.
    listed.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(listed)
}

/// Dispositivo resuelto y listo para abrir.
pub struct ResolvedDevice {
    pub device: Device,
    pub info: AudioDeviceInfo,
    pub resolution: DeviceResolution,
}

/// Resuelve la preferencia guardada siguiendo el orden de §18:
/// id estable, luego nombre, luego predeterminado del sistema.
pub fn resolve_device(
    preferred_id: Option<&str>,
    preferred_name: Option<&str>,
) -> AppResult<ResolvedDevice> {
    let host = cpal::default_host();
    let default_device = host.default_output_device();
    let default_id = default_device.as_ref().and_then(|device| device.id().ok());

    if let Some(preferred_id) = preferred_id {
        if let Ok(parsed) = DeviceId::from_str(preferred_id) {
            if let Some(device) = host.device_by_id(&parsed) {
                let info = describe(&device, default_id.as_ref());
                return Ok(ResolvedDevice {
                    device,
                    info,
                    resolution: DeviceResolution::ExactId,
                });
            }
        }
    }

    if let Some(preferred_name) = preferred_name {
        let found = host.output_devices().ok().and_then(|mut devices| {
            devices.find(|device| {
                device
                    .description()
                    .map(|description| description.name() == preferred_name)
                    .unwrap_or(false)
            })
        });

        if let Some(device) = found {
            let info = describe(&device, default_id.as_ref());
            return Ok(ResolvedDevice {
                device,
                info,
                resolution: DeviceResolution::ByName,
            });
        }
    }

    let device = default_device.ok_or_else(|| {
        AppError::new(
            ErrorKind::AudioDevice,
            "El sistema no reporta ningun dispositivo de salida de audio disponible.",
        )
    })?;
    let info = describe(&device, default_id.as_ref());
    let had_preference = preferred_id.is_some() || preferred_name.is_some();

    Ok(ResolvedDevice {
        device,
        info,
        resolution: if had_preference {
            DeviceResolution::FallbackDefault
        } else {
            DeviceResolution::SystemDefault
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_aviso_solo_aparece_cuando_no_se_respeto_la_eleccion() {
        assert_eq!(DeviceResolution::ExactId.notice(Some("CABLE Input")), None);
        assert_eq!(DeviceResolution::SystemDefault.notice(None), None);
        assert!(DeviceResolution::ByName.notice(None).is_some());

        let aviso = DeviceResolution::FallbackDefault
            .notice(Some("CABLE Input"))
            .expect("debe avisar");
        assert!(aviso.contains("CABLE Input"));
        assert!(aviso.contains("predeterminado"));
    }

    /// Enumerar dispositivos depende del host de audio de la maquina. En CI sin
    /// tarjeta de sonido puede devolver una lista vacia, pero nunca debe
    /// entrar en panico ni colgarse.
    #[test]
    fn listar_dispositivos_no_entra_en_panico() {
        match list_output_devices() {
            Ok(devices) => {
                assert!(devices.iter().filter(|d| d.is_default).count() <= 1);
                for device in &devices {
                    assert!(!device.name.is_empty());
                }
            }
            Err(error) => assert_eq!(error.kind, ErrorKind::AudioDevice),
        }
    }
}
