//! Paginas y slots de la botonera.

use serde::{Deserialize, Serialize};

use super::sound::Sound;

/// Cantidad fija de slots por pagina (§43).
pub const SLOTS_PER_PAGE: u8 = 9;
/// Maximo inicial de paginas (§43).
pub const MAX_PAGES: usize = 9;

/// Numero de slot validado en el rango 1..=9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct SlotNumber(u8);

impl SlotNumber {
    pub fn new(value: u8) -> Result<Self, String> {
        if (1..=SLOTS_PER_PAGE).contains(&value) {
            Ok(Self(value))
        } else {
            Err(format!(
                "El numero de slot debe estar entre 1 y {SLOTS_PER_PAGE}, se recibio {value}."
            ))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }

    /// Los nueve slots en orden.
    pub fn all() -> impl Iterator<Item = SlotNumber> {
        (1..=SLOTS_PER_PAGE).map(SlotNumber)
    }
}

impl TryFrom<u8> for SlotNumber {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        SlotNumber::new(value)
    }
}

impl From<SlotNumber> for u8 {
    fn from(value: SlotNumber) -> Self {
        value.0
    }
}

/// Slot de una pagina, con el sonido resuelto.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundSlot {
    pub page_id: String,
    pub slot_number: SlotNumber,
    pub sound: Option<Sound>,
    pub custom_label: Option<String>,
    pub custom_volume: Option<f32>,
}

impl SoundSlot {
    pub fn empty(page_id: &str, slot_number: SlotNumber) -> Self {
        Self {
            page_id: page_id.to_string(),
            slot_number,
            sound: None,
            custom_label: None,
            custom_volume: None,
        }
    }

    /// Etiqueta mostrada en el boton: la personalizada tiene prioridad.
    pub fn display_label(&self) -> Option<&str> {
        self.custom_label
            .as_deref()
            .or_else(|| self.sound.as_ref().map(|sound| sound.name.as_str()))
    }
}

/// Pagina con sus nueve slots, siempre completos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPage {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub slots: Vec<SoundSlot>,
}

/// Resumen de pagina sin resolver los sonidos (para listados y el selector).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSummary {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub assigned_slots: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_rango_de_slots() {
        assert!(SlotNumber::new(0).is_err());
        assert!(SlotNumber::new(10).is_err());
        for n in 1..=9u8 {
            assert_eq!(SlotNumber::new(n).unwrap().get(), n);
        }
    }

    #[test]
    fn all_devuelve_nueve_slots_ordenados() {
        let slots: Vec<u8> = SlotNumber::all().map(SlotNumber::get).collect();
        assert_eq!(slots, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn deserializa_rechazando_valores_invalidos() {
        assert!(serde_json::from_str::<SlotNumber>("5").is_ok());
        assert!(serde_json::from_str::<SlotNumber>("0").is_err());
        assert!(serde_json::from_str::<SlotNumber>("12").is_err());
    }

    #[test]
    fn etiqueta_personalizada_tiene_prioridad() {
        let mut slot = SoundSlot::empty("page-1", SlotNumber::new(1).unwrap());
        assert_eq!(slot.display_label(), None);
        slot.custom_label = Some("Bruh".into());
        assert_eq!(slot.display_label(), Some("Bruh"));
    }
}
