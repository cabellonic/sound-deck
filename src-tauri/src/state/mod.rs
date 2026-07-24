//! Estado compartido de la aplicacion, accesible desde los comandos Tauri.

use std::collections::HashMap;

use parking_lot::Mutex;
use tauri::AppHandle;

use crate::audio::AudioEngine;
use crate::database::{settings as settings_repo, Database};
use crate::domain::settings::AppSettings;
use crate::errors::AppResult;
use crate::filesystem::AppPaths;
use crate::overlay::OverlayState;
use crate::providers::{registry::ProviderRegistry, RemoteSound};
use crate::shortcuts::ShortcutRegistry;

/// Cuantos resultados online recordamos para poder resolverlos por id.
/// Es el "store temporal" de §25: por el `dataTransfer` solo viajan ids.
const REMOTE_CACHE_LIMIT: usize = 600;

pub struct AppState {
    pub db: Database,
    pub paths: AppPaths,
    pub audio: AudioEngine,
    pub overlay: OverlayState,
    pub shortcuts: ShortcutRegistry,
    pub providers: ProviderRegistry,
    pub http: reqwest::Client,
    /// Ultimos resultados online vistos, indexados por `proveedor:idRemoto`.
    remote_cache: Mutex<HashMap<String, RemoteSound>>,
    /// Pagina activa, compartida entre la ventana principal y el overlay.
    active_page: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(app: &AppHandle, db: Database, paths: AppPaths, http: reqwest::Client) -> Self {
        Self {
            audio: AudioEngine::new(app.clone()),
            providers: ProviderRegistry::new(http.clone()),
            overlay: OverlayState::new(),
            shortcuts: ShortcutRegistry::new(),
            remote_cache: Mutex::new(HashMap::new()),
            active_page: Mutex::new(None),
            db,
            paths,
            http,
        }
    }

    pub fn settings(&self) -> AppResult<AppSettings> {
        settings_repo::load(&self.db)
    }

    /// Guarda resultados online para poder resolverlos luego por identificador.
    pub fn cache_remote_sounds(&self, sounds: &[RemoteSound]) {
        let mut cache = self.remote_cache.lock();

        // Evitamos que el cache crezca sin limite en una sesion larga.
        if cache.len() + sounds.len() > REMOTE_CACHE_LIMIT {
            cache.clear();
        }

        for sound in sounds {
            cache.insert(
                remote_key(&sound.provider_id, &sound.remote_id),
                sound.clone(),
            );
        }
    }

    pub fn remote_sound(&self, provider_id: &str, remote_id: &str) -> Option<RemoteSound> {
        self.remote_cache
            .lock()
            .get(&remote_key(provider_id, remote_id))
            .cloned()
    }

    pub fn active_page(&self) -> Option<String> {
        self.active_page.lock().clone()
    }

    pub fn set_active_page(&self, page_id: Option<String>) {
        *self.active_page.lock() = page_id;
    }
}

/// Clave del cache de resultados online.
///
/// Usamos el separador de unidad ASCII en lugar de `:` porque un id que
/// contuviera dos puntos podria colisionar con otro par distinto.
fn remote_key(provider_id: &str, remote_id: &str) -> String {
    format!("{provider_id}\u{1f}{remote_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remoto(id: &str) -> RemoteSound {
        RemoteSound {
            provider_id: "freesound".into(),
            remote_id: id.into(),
            title: format!("Sonido {id}"),
            description: None,
            duration_ms: None,
            preview_url: None,
            source_page_url: None,
            download_reference: None,
            provider_category: None,
            normalized_category: None,
            tags: vec![],
            license: None,
            attribution: None,
            file_size_bytes: None,
        }
    }

    /// El cache es logica pura: se prueba sin construir la aplicacion Tauri.
    #[test]
    fn la_clave_combina_proveedor_e_id_sin_ambiguedad() {
        assert_eq!(remote_key("freesound", "42"), "freesound\u{1f}42");
        // Un id con dos puntos no debe poder colisionar con otro par distinto.
        assert_ne!(remote_key("a", "b:c"), remote_key("a:b", "c"));
    }

    #[test]
    fn el_cache_guarda_y_recupera_por_identificador() {
        let cache: Mutex<HashMap<String, RemoteSound>> = Mutex::new(HashMap::new());
        let sonido = remoto("42");
        cache
            .lock()
            .insert(remote_key(&sonido.provider_id, &sonido.remote_id), sonido);

        assert!(cache.lock().contains_key(&remote_key("freesound", "42")));
        assert!(!cache.lock().contains_key(&remote_key("freesound", "99")));
    }
}
