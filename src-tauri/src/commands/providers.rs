//! Comandos de proveedores online: busqueda, descarga y configuracion (§11-§14).

use tauri::{AppHandle, Manager, State};

use crate::database::provider_settings;
use crate::domain::{new_id, Sound, SoundSource};
use crate::errors::{AppError, AppResult, ErrorKind};
use crate::events::{
    self, DownloadCompletedPayload, DownloadFailedPayload, DownloadProgressPayload,
};
use crate::library::{self, IngestRequest, SourceHandling};
use crate::providers::registry::ProviderStatus;
use crate::providers::{oauth, RemoteSound, SearchOptions};
use crate::state::AppState;

#[tauri::command(async)]
pub fn list_providers(state: State<'_, AppState>) -> AppResult<Vec<ProviderStatus>> {
    state.providers.statuses(&state.db)
}

#[tauri::command(async)]
pub fn set_provider_enabled(
    state: State<'_, AppState>,
    provider_id: String,
    enabled: bool,
) -> AppResult<Vec<ProviderStatus>> {
    if state.providers.get(&provider_id).is_none() {
        return Err(AppError::not_found("Ese proveedor no existe."));
    }
    provider_settings::set_enabled(&state.db, &provider_id, enabled)?;
    state.providers.statuses(&state.db)
}

/// Guarda la API key de un proveedor. La clave nunca vuelve al frontend.
#[tauri::command(async)]
pub fn set_provider_api_key(
    state: State<'_, AppState>,
    provider_id: String,
    api_key: Option<String>,
) -> AppResult<Vec<ProviderStatus>> {
    if state.providers.get(&provider_id).is_none() {
        return Err(AppError::not_found("Ese proveedor no existe."));
    }
    provider_settings::set_api_key(&state.db, &provider_id, api_key.as_deref())?;
    state.providers.statuses(&state.db)
}

/// Guarda el client id de OAuth2 del proveedor.
#[tauri::command(async)]
pub fn set_provider_client_id(
    state: State<'_, AppState>,
    provider_id: String,
    client_id: Option<String>,
) -> AppResult<Vec<ProviderStatus>> {
    if state.providers.get(&provider_id).is_none() {
        return Err(AppError::not_found("Ese proveedor no existe."));
    }
    provider_settings::set_client_id(&state.db, &provider_id, client_id.as_deref())?;
    state.providers.statuses(&state.db)
}

/// URL a la que hay que mandar al usuario para autorizar la aplicacion.
///
/// El `state` vuelve junto a la URL para que la interfaz pueda mostrarlo si
/// hace falta; en un flujo donde el codigo se pega a mano no hay nada que
/// validar automaticamente.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub url: String,
    pub state: String,
}

#[tauri::command(async)]
pub fn begin_provider_authorization(
    state: State<'_, AppState>,
    provider_id: String,
) -> AppResult<AuthorizationRequest> {
    let record = provider_settings::get(&state.db, &provider_id)?;
    let client_id = record
        .and_then(|record| record.config.client_id)
        .ok_or_else(|| {
            AppError::validation(
                "Falta el Client id de la credencial. Copialo desde tu aplicacion de Freesound.",
            )
        })?;

    let nonce = crate::domain::new_id();
    Ok(AuthorizationRequest {
        url: oauth::authorize_url(&client_id, &nonce),
        state: nonce,
    })
}

/// Canjea el codigo que el usuario copio de la pagina de Freesound.
#[tauri::command]
pub async fn complete_provider_authorization(
    app: AppHandle,
    provider_id: String,
    code: String,
) -> AppResult<Vec<ProviderStatus>> {
    let code = code.trim().to_string();
    if code.is_empty() {
        return Err(AppError::validation("Pega el codigo de autorizacion."));
    }

    let (client, db, providers, client_id, secret) = {
        let state = app.state::<AppState>();
        let record = provider_settings::get(&state.db, &provider_id)?;
        let config = record.map(|record| record.config).unwrap_or_default();

        let client_id = config.client_id.ok_or_else(|| {
            AppError::validation("Falta el Client id de la credencial de Freesound.")
        })?;
        let secret = config.api_key.ok_or_else(|| {
            AppError::validation("Falta la API key, que Freesound usa tambien como Client secret.")
        })?;

        (
            state.http.clone(),
            state.db.clone(),
            state.providers.clone(),
            client_id,
            secret,
        )
    };

    let tokens = oauth::exchange_code(&client, &client_id, &secret, &code)
        .await
        .map_err(|error| {
            let technical = error.to_string();
            let message = match error {
                // Un rechazo aca casi nunca es la credencial: es el codigo, que
                // dura diez minutos y sirve una sola vez.
                crate::providers::ProviderError::Unauthorized => {
                    "El codigo no es valido o ya vencio. Duran 10 minutos y sirven una sola vez: volve a autorizar."
                        .to_string()
                }
                other => other.user_message("Freesound"),
            };
            AppError::new(ErrorKind::Provider, message).with_technical(technical)
        })?;

    provider_settings::set_oauth_tokens(&db, &provider_id, Some(tokens))?;
    tracing::info!(provider_id, "cuenta conectada por OAuth2");
    providers.statuses(&db)
}

/// Desconecta la cuenta y borra los tokens guardados.
#[tauri::command(async)]
pub fn disconnect_provider_account(
    state: State<'_, AppState>,
    provider_id: String,
) -> AppResult<Vec<ProviderStatus>> {
    provider_settings::set_oauth_tokens(&state.db, &provider_id, None)?;
    tracing::info!(provider_id, "cuenta desconectada");
    state.providers.statuses(&state.db)
}

#[tauri::command]
pub async fn test_provider_connection(app: AppHandle, provider_id: String) -> AppResult<()> {
    let (provider, context) = {
        let state = app.state::<AppState>();
        let provider = state
            .providers
            .get(&provider_id)
            .ok_or_else(|| AppError::not_found("Ese proveedor no existe."))?;
        let context = state.providers.context_for(&state.db, &provider_id)?;
        (provider, context)
    };

    provider.test_connection(&context).await.map_err(|error| {
        AppError::new(
            ErrorKind::Provider,
            error.user_message(provider.display_name()),
        )
        .with_technical(error.to_string())
        .with_detail("providerId", provider_id)
    })
}

/// Resultados de un proveedor, o su error, sin cortar a los demas (§13).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSearchResult {
    pub provider_id: String,
    pub provider_name: String,
    pub items: Vec<RemoteSound>,
    pub has_more: bool,
    pub total: Option<u64>,
    /// Mensaje de error si este proveedor concreto fallo.
    pub error: Option<String>,
}

/// Busca en todos los proveedores habilitados.
///
/// Cada proveedor se consulta por separado: si uno falla o esta caido, los
/// demas siguen mostrando resultados y el error se informa solo para ese (§13).
#[tauri::command]
pub async fn search_remote_sounds(
    app: AppHandle,
    query: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> AppResult<Vec<ProviderSearchResult>> {
    let providers = {
        let state = app.state::<AppState>();
        state.providers.enabled(&state.db)?
    };

    if providers.is_empty() {
        return Err(AppError::new(
            ErrorKind::Configuration,
            "No hay proveedores online configurados. Activa uno en Ajustes > Proveedores.",
        ));
    }

    let options = SearchOptions {
        page: page.unwrap_or(1).max(1),
        page_size: page_size.unwrap_or(30).clamp(1, 100),
    };

    // Los proveedores se consultan en paralelo: uno lento (o con rate limit
    // propio, como MyInstants) no debe retrasar los resultados de los demas.
    // Cada uno mantiene su propio limite de frecuencia internamente.
    let searches = providers.into_iter().map(|(provider, context)| {
        let query = query.clone();
        async move {
            let outcome = provider.search(&query, options, &context).await;
            (provider, outcome)
        }
    });

    let results = futures_util::future::join_all(searches)
        .await
        .into_iter()
        .map(|(provider, outcome)| match outcome {
            Ok(page) => {
                app.state::<AppState>().cache_remote_sounds(&page.items);
                ProviderSearchResult {
                    provider_id: provider.id().to_string(),
                    provider_name: provider.display_name().to_string(),
                    items: page.items,
                    has_more: page.has_more,
                    total: page.total,
                    error: None,
                }
            }
            Err(error) => {
                // El fallo de un proveedor se informa solo para ese proveedor;
                // el resto sigue mostrando resultados (§13).
                tracing::warn!(
                    provider = provider.id(),
                    technical = %error,
                    "el proveedor fallo durante la busqueda"
                );
                ProviderSearchResult {
                    provider_id: provider.id().to_string(),
                    provider_name: provider.display_name().to_string(),
                    items: Vec::new(),
                    has_more: false,
                    total: None,
                    error: Some(error.user_message(provider.display_name())),
                }
            }
        })
        .collect();

    Ok(results)
}

/// Descarga un resultado online y lo guarda en la biblioteca.
///
/// El proceso completo esta en `download_remote`: resolver, validar la URL,
/// bajar a temporal, validar el contenido, deduplicar, mover e insertar.
#[tauri::command]
pub async fn download_remote_sound(
    app: AppHandle,
    provider_id: String,
    remote_id: String,
) -> AppResult<Sound> {
    let operation_id = new_id();
    download_remote(&app, &provider_id, &remote_id, &operation_id).await
}

/// Descarga y asigna en una sola operacion, para el drag desde Internet a un
/// slot (§7). Si algo falla, el slot queda como estaba: nunca apuntando a un
/// archivo inexistente.
#[tauri::command]
pub async fn download_and_assign_remote_sound(
    app: AppHandle,
    provider_id: String,
    remote_id: String,
    page_id: String,
    slot_number: crate::domain::SlotNumber,
) -> AppResult<crate::domain::SoundSlot> {
    let operation_id = new_id();
    let sound = download_remote(&app, &provider_id, &remote_id, &operation_id).await?;

    let state = app.state::<AppState>();
    let slot = crate::database::slots::assign(&state.db, &page_id, slot_number, &sound.id)?;

    events::emit(
        &app,
        events::SLOT_CHANGED,
        events::SlotChangedPayload {
            page_id,
            slot_number: slot_number.get(),
        },
    );
    Ok(slot)
}

async fn download_remote(
    app: &AppHandle,
    provider_id: &str,
    remote_id: &str,
    operation_id: &str,
) -> AppResult<Sound> {
    let result = download_remote_inner(app, provider_id, remote_id, operation_id).await;

    match &result {
        Ok(sound) => events::emit(
            app,
            events::DOWNLOAD_COMPLETED,
            DownloadCompletedPayload {
                operation_id: operation_id.to_string(),
                provider_id: provider_id.to_string(),
                remote_id: remote_id.to_string(),
                sound_id: sound.id.clone(),
                deduplicated: false,
            },
        ),
        Err(error) => events::emit(
            app,
            events::DOWNLOAD_FAILED,
            DownloadFailedPayload {
                operation_id: operation_id.to_string(),
                provider_id: provider_id.to_string(),
                remote_id: remote_id.to_string(),
                message: error.message.clone(),
            },
        ),
    }

    result
}

async fn download_remote_inner(
    app: &AppHandle,
    provider_id: &str,
    remote_id: &str,
    operation_id: &str,
) -> AppResult<Sound> {
    let (provider, remote, client, max_bytes) = {
        let state = app.state::<AppState>();
        let provider = state
            .providers
            .get(provider_id)
            .ok_or_else(|| AppError::not_found("Ese proveedor no esta disponible."))?;
        let remote = state.remote_sound(provider_id, remote_id).ok_or_else(|| {
            AppError::not_found("Ese resultado ya no esta disponible. Repeti la busqueda.")
        })?;
        let settings = state.settings()?;
        (
            provider,
            remote,
            state.http.clone(),
            settings.audio.max_download_bytes,
        )
    };

    // Si ya lo tenemos guardado, no volvemos a bajarlo (§39 "dos descargas del
    // mismo audio").
    {
        let state = app.state::<AppState>();
        if let Some(existing) =
            crate::database::sounds::find_by_remote(&state.db, provider_id, remote_id)?
        {
            if existing.file_available {
                tracing::info!(sound_id = %existing.id, "el audio remoto ya estaba descargado");
                return Ok(existing);
            }
        }
    }

    // Se resuelve aca y no dentro del bloque de arriba porque renovar el token
    // OAuth2 es asincronico y el estado de Tauri no cruza un `await`.
    let context = {
        let (providers, db) = {
            let state = app.state::<AppState>();
            (state.providers.clone(), state.db.clone())
        };
        providers
            .download_context(&db, &client, provider_id)
            .await?
    };

    let resolved = provider
        .resolve_download(&remote, &context)
        .await
        .map_err(|error| {
            AppError::new(
                ErrorKind::Provider,
                error.user_message(provider.display_name()),
            )
            .with_technical(error.to_string())
        })?;

    let allowed: Vec<&str> = resolved.allowed_hosts.iter().map(String::as_str).collect();
    crate::downloads::url::validate_remote_url(&resolved.url, &allowed)?;

    let temp_path = {
        let state = app.state::<AppState>();
        state
            .paths
            .new_temp_file(resolved.suggested_extension.as_deref().unwrap_or("part"))
    };

    let progress = {
        let app = app.clone();
        let operation_id = operation_id.to_string();
        let provider_id = provider_id.to_string();
        let remote_id = remote_id.to_string();
        move |received: u64, total: Option<u64>| {
            events::emit(
                &app,
                events::DOWNLOAD_PROGRESS,
                DownloadProgressPayload {
                    operation_id: operation_id.clone(),
                    provider_id: provider_id.clone(),
                    remote_id: remote_id.clone(),
                    received_bytes: received,
                    total_bytes: total,
                },
            );
        }
    };

    crate::downloads::download_to_temp_with_headers(
        &client,
        &resolved.url,
        &resolved.headers,
        &temp_path,
        max_bytes,
        Some(&progress),
    )
    .await?;

    // Validar, hashear e insertar es trabajo bloqueante: fuera del runtime async.
    let app_for_blocking = app.clone();
    let request = IngestRequest {
        source_path: temp_path,
        handling: SourceHandling::Move,
        display_name: Some(remote.title.clone()),
        source: SoundSource::Provider {
            provider_id: provider_id.to_string(),
            remote_id: remote_id.to_string(),
        },
        source_page_url: remote.source_page_url.clone(),
        download_url_reference: remote.download_reference.clone(),
        provider_category: remote.provider_category.clone(),
        normalized_category: remote.normalized_category,
        license: resolved.license.clone().or(remote.license.clone()),
        attribution: resolved.attribution.clone().or(remote.attribution.clone()),
        tags: remote.tags.clone(),
    };

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_blocking.state::<AppState>();
        library::ingest(&state.db, &state.paths, request, max_bytes)
    })
    .await
    .map_err(|error| {
        AppError::new(ErrorKind::Download, "La descarga se interrumpio.")
            .with_technical(error.to_string())
    })??;

    events::emit(app, events::LIBRARY_CHANGED, ());
    Ok(outcome.sound().clone())
}
