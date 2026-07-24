//! Motor de audio: enumeracion de dispositivos y reproduccion.

pub mod devices;
pub mod engine;

pub use devices::{list_output_devices, AudioDeviceInfo, DeviceResolution};
pub use engine::AudioEngine;
