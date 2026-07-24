//! Sound Deck: soundboard de escritorio con overlay, biblioteca local y
//! busqueda en proveedores online.
//!
//! ## Organizacion
//!
//! - `domain`: tipos compartidos, sin dependencias de infraestructura.
//! - `database`: SQLite (rusqlite) y repositorios.
//! - `filesystem`: rutas administradas y validacion de archivos de audio.
//! - `library`: unica puerta de entrada de audios a la biblioteca.
//! - `audio`: dispositivos y motor de reproduccion.
//! - `providers`: proveedores online detras de un trait.
//! - `downloads`: descarga con limites y validacion de URL.
//! - `shortcuts`, `overlay`, `tray`, `platform`: integracion con el sistema.
//! - `commands`: la API que ve el frontend.

pub mod app;
pub mod audio;
pub mod commands;
pub mod database;
pub mod domain;
pub mod downloads;
pub mod errors;
pub mod events;
pub mod filesystem;
pub mod library;
pub mod logging;
pub mod overlay;
pub mod platform;
pub mod providers;
pub mod shortcuts;
pub mod state;
pub mod tray;

pub use app::run;
