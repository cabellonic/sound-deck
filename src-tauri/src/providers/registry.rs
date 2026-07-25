//! Registro de proveedores disponibles y su estado de configuracion.

use std::sync::Arc;

use serde::Serialize;

use crate::database::{provider_settings, Database};
use crate::errors::AppResult;

use super::freesound::FreesoundProvider;
use super::myinstants::MyInstantsProvider;
use super::{oauth, ProviderContext, SoundProvider};

/// Estado de un proveedor tal como lo muestra la pantalla de ajustes (§20).
/// Nunca incluye la API key completa.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub id: String,
    pub display_name: String,
    pub homepage: String,
    pub enabled: bool,
    pub requires_api_key: bool,
    pub has_api_key: bool,
    /// Clave enmascarada, solo para confirmar visualmente que hay una guardada.
    pub masked_api_key: Option<String>,
    pub unofficial: bool,
    pub supports_preview: bool,
    pub supports_download: bool,
    /// `true` cuando el proveedor puede usarse ahora mismo.
    pub ready: bool,
    /// Si el proveedor puede conectar una cuenta por OAuth2 para descargar el
    /// archivo original en vez de la preview.
    pub supports_oauth: bool,
    pub has_client_id: bool,
    /// Si hay una cuenta conectada ahora mismo.
    pub account_connected: bool,
}

/// Todos los proveedores compilados en la aplicacion.
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Vec<Arc<dyn SoundProvider>>,
}

impl ProviderRegistry {
    /// Todos los proveedores quedan compilados en el binario, pero ninguno se
    /// consulta hasta que el usuario lo habilita en Ajustes. Los no oficiales
    /// se marcan como tales para que la interfaz lo advierta (§12).
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            providers: vec![
                Arc::new(FreesoundProvider::new(client.clone())),
                Arc::new(MyInstantsProvider::new(client)),
            ],
        }
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn SoundProvider>> {
        self.providers
            .iter()
            .find(|provider| provider.id() == provider_id)
            .cloned()
    }

    pub fn all(&self) -> &[Arc<dyn SoundProvider>] {
        &self.providers
    }

    /// Proveedores habilitados y con todo lo que necesitan para funcionar.
    /// La busqueda online solo consulta estos.
    pub fn enabled(
        &self,
        db: &Database,
    ) -> AppResult<Vec<(Arc<dyn SoundProvider>, ProviderContext)>> {
        let mut ready = Vec::new();

        for provider in &self.providers {
            let record = provider_settings::get(db, provider.id())?;
            let Some(record) = record else { continue };
            if !record.enabled {
                continue;
            }

            // La busqueda no necesita la cuenta conectada: el token se resuelve
            // solo cuando hace falta, en `context_for`.
            let context = ProviderContext {
                api_key: record.config.api_key.clone(),
                access_token: None,
            };

            if provider.capabilities().requires_api_key && context.api_key.is_none() {
                continue;
            }

            ready.push((provider.clone(), context));
        }

        Ok(ready)
    }

    /// Contexto (credenciales) de un proveedor concreto, sin cuenta conectada.
    pub fn context_for(&self, db: &Database, provider_id: &str) -> AppResult<ProviderContext> {
        let record = provider_settings::get(db, provider_id)?;
        Ok(ProviderContext {
            api_key: record.and_then(|record| record.config.api_key),
            access_token: None,
        })
    }

    /// Contexto con el token OAuth2 al dia, renovandolo si hacia falta.
    ///
    /// Es el que usa la descarga. Que la cuenta no este conectada no es un
    /// error: el proveedor cae a lo que pueda dar sin ella. Que el refresco
    /// falle tampoco corta la descarga, pero si borra los tokens muertos para
    /// que la interfaz deje de decir que la cuenta esta conectada.
    pub async fn download_context(
        &self,
        db: &Database,
        client: &reqwest::Client,
        provider_id: &str,
    ) -> AppResult<ProviderContext> {
        let Some(record) = provider_settings::get(db, provider_id)? else {
            return Ok(ProviderContext::default());
        };

        let api_key = record.config.api_key.clone();
        let mut access_token = None;

        if let (Some(tokens), Some(client_id), Some(secret)) = (
            record.config.oauth.clone(),
            record.config.client_id.clone(),
            api_key.clone(),
        ) {
            access_token = if tokens.is_fresh() {
                Some(tokens.access_token)
            } else {
                match oauth::refresh(client, &client_id, &secret, &tokens).await {
                    Ok(renewed) => {
                        provider_settings::set_oauth_tokens(
                            db,
                            provider_id,
                            Some(renewed.clone()),
                        )?;
                        Some(renewed.access_token)
                    }
                    Err(error) => {
                        tracing::warn!(
                            provider_id,
                            %error,
                            "no se pudo renovar el acceso a la cuenta; se desconecta"
                        );
                        provider_settings::set_oauth_tokens(db, provider_id, None)?;
                        None
                    }
                }
            };
        }

        Ok(ProviderContext {
            api_key,
            access_token,
        })
    }

    /// Estado de todos los proveedores para la pantalla de ajustes.
    pub fn statuses(&self, db: &Database) -> AppResult<Vec<ProviderStatus>> {
        let mut statuses = Vec::new();

        for provider in &self.providers {
            let record = provider_settings::get(db, provider.id())?;
            let capabilities = provider.capabilities();
            let api_key = record
                .as_ref()
                .and_then(|record| record.config.api_key.clone());
            let enabled = record
                .as_ref()
                .map(|record| record.enabled)
                .unwrap_or(false);
            let has_api_key = api_key.is_some();

            statuses.push(ProviderStatus {
                id: provider.id().to_string(),
                display_name: provider.display_name().to_string(),
                homepage: provider.homepage().to_string(),
                enabled,
                requires_api_key: capabilities.requires_api_key,
                has_api_key,
                masked_api_key: api_key.as_deref().map(provider_settings::mask_secret),
                unofficial: capabilities.unofficial,
                supports_preview: capabilities.preview,
                supports_download: capabilities.download,
                ready: enabled && (!capabilities.requires_api_key || has_api_key),
                supports_oauth: capabilities.oauth,
                has_client_id: record
                    .as_ref()
                    .is_some_and(|record| record.config.client_id.is_some()),
                account_connected: record
                    .as_ref()
                    .is_some_and(|record| record.config.oauth.is_some()),
            });
        }

        Ok(statuses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_db;

    fn registry() -> ProviderRegistry {
        ProviderRegistry::new(reqwest::Client::new())
    }

    fn estado(registry: &ProviderRegistry, db: &Database, id: &str) -> ProviderStatus {
        registry
            .statuses(db)
            .unwrap()
            .into_iter()
            .find(|status| status.id == id)
            .unwrap_or_else(|| panic!("falta el proveedor {id}"))
    }

    /// Conectar y desconectar la cuenta se tiene que reflejar en el estado que
    /// ve Ajustes, y nunca puede filtrar el token.
    #[test]
    fn el_estado_refleja_la_cuenta_conectada_sin_exponer_el_token() {
        let db = test_db();
        let registry = registry();
        let tokens = crate::providers::oauth::OAuthTokens {
            access_token: "secreto-de-acceso".into(),
            refresh_token: "secreto-de-refresco".into(),
            expires_at: crate::providers::oauth::now_seconds() + 3600,
        };

        provider_settings::set_api_key(&db, "freesound", Some("clave")).unwrap();
        provider_settings::set_client_id(&db, "freesound", Some("mi-client-id")).unwrap();

        let sin_cuenta = estado(&registry, &db, "freesound");
        assert!(sin_cuenta.supports_oauth);
        assert!(sin_cuenta.has_client_id);
        assert!(!sin_cuenta.account_connected);

        provider_settings::set_oauth_tokens(&db, "freesound", Some(tokens)).unwrap();
        let conectada = estado(&registry, &db, "freesound");
        assert!(conectada.account_connected);
        // Guardar los tokens no debe haber pisado el resto de la configuracion.
        assert!(conectada.has_api_key);
        assert!(conectada.has_client_id);

        let serializado = serde_json::to_string(&conectada).unwrap();
        assert!(!serializado.contains("secreto"), "{serializado}");

        provider_settings::set_oauth_tokens(&db, "freesound", None).unwrap();
        assert!(!estado(&registry, &db, "freesound").account_connected);
    }

    #[test]
    fn un_proveedor_sin_oauth_no_ofrece_conectar_cuenta() {
        let db = test_db();
        let registry = registry();
        let estado = estado(&registry, &db, "myinstants");
        assert!(!estado.supports_oauth);
        assert!(!estado.account_connected);
    }

    #[test]
    fn los_proveedores_esperados_estan_registrados() {
        let registry = registry();
        assert!(registry.get("freesound").is_some());
        assert!(registry.get("myinstants").is_some());
        assert!(registry.get("inexistente").is_none());
    }

    #[test]
    fn ninguno_esta_habilitado_al_instalar() {
        let db = test_db();
        let registry = registry();
        assert!(registry.enabled(&db).unwrap().is_empty());

        for status in registry.statuses(&db).unwrap() {
            assert!(!status.enabled, "{} no deberia venir habilitado", status.id);
            assert!(!status.ready);
            assert!(status.masked_api_key.is_none());
        }
    }

    #[test]
    fn habilitado_pero_sin_clave_no_se_consulta() {
        let db = test_db();
        let registry = registry();
        provider_settings::set_enabled(&db, "freesound", true).unwrap();

        assert!(registry.enabled(&db).unwrap().is_empty());
        assert!(!estado(&registry, &db, "freesound").ready);
    }

    #[test]
    fn habilitado_y_con_clave_queda_listo() {
        let db = test_db();
        let registry = registry();
        provider_settings::set_enabled(&db, "freesound", true).unwrap();
        provider_settings::set_api_key(&db, "freesound", Some("clave-secreta-larga")).unwrap();

        let habilitados = registry.enabled(&db).unwrap();
        assert_eq!(habilitados.len(), 1);
        assert_eq!(habilitados[0].0.id(), "freesound");

        let status = estado(&registry, &db, "freesound");
        assert!(status.ready);
        assert!(status.has_api_key);
        // La clave nunca sale completa hacia el frontend.
        let enmascarada = status.masked_api_key.as_deref().unwrap();
        assert!(!enmascarada.contains("clave-secreta"));
        assert!(enmascarada.starts_with('\u{2022}'));
    }

    #[test]
    fn un_proveedor_sin_api_key_queda_listo_apenas_se_habilita() {
        let db = test_db();
        let registry = registry();

        let status = estado(&registry, &db, "myinstants");
        assert!(status.unofficial, "debe marcarse como no oficial");
        assert!(!status.requires_api_key);
        assert!(!status.ready, "desactivado por defecto");

        provider_settings::set_enabled(&db, "myinstants", true).unwrap();

        assert!(estado(&registry, &db, "myinstants").ready);
        assert_eq!(registry.enabled(&db).unwrap().len(), 1);
    }

    #[test]
    fn desactivar_un_proveedor_lo_saca_de_las_consultas() {
        let db = test_db();
        let registry = registry();
        provider_settings::set_enabled(&db, "myinstants", true).unwrap();
        assert_eq!(registry.enabled(&db).unwrap().len(), 1);

        provider_settings::set_enabled(&db, "myinstants", false).unwrap();
        assert!(registry.enabled(&db).unwrap().is_empty());
    }
}
