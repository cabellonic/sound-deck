//! Comandos de paginas y slots (§23).

use tauri::{AppHandle, State};

use crate::database::{pages, settings as settings_repo, slots};
use crate::domain::{PageSummary, SlotNumber, SoundPage, SoundSlot};
use crate::errors::{AppError, AppResult};
use crate::events::{self, SlotChangedPayload};
use crate::state::AppState;

#[tauri::command(async)]
pub fn list_pages(state: State<'_, AppState>) -> AppResult<Vec<PageSummary>> {
    pages::list_summaries(&state.db)
}

/// Devuelve una pagina con sus nueve slots resueltos.
/// Sin `page_id`, devuelve la ultima activa o la primera disponible.
#[tauri::command(async)]
pub fn get_page(state: State<'_, AppState>, page_id: Option<String>) -> AppResult<SoundPage> {
    let requested = page_id.or_else(|| state.active_page()).or_else(|| {
        settings_repo::load(&state.db)
            .ok()
            .and_then(|settings| settings.general.last_page_id)
    });

    // Si la pagina pedida ya no existe (fue borrada mientras estaba activa),
    // caemos a la primera en lugar de fallar (§39).
    let page = match requested {
        Some(id) => match pages::get(&state.db, &id)? {
            Some(page) => Some(page),
            None => pages::first(&state.db)?,
        },
        None => pages::first(&state.db)?,
    };

    let page = page.ok_or_else(|| {
        AppError::not_found("No hay ninguna pagina disponible. Crea una para empezar.")
    })?;

    state.set_active_page(Some(page.id.clone()));
    let _ = settings_repo::save_last_page(&state.db, &page.id);
    Ok(page)
}

#[tauri::command(async)]
pub fn set_active_page(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
) -> AppResult<SoundPage> {
    let page = pages::get(&state.db, &page_id)?
        .ok_or_else(|| AppError::not_found("Esa pagina ya no existe."))?;

    state.set_active_page(Some(page.id.clone()));
    let _ = settings_repo::save_last_page(&state.db, &page.id);
    events::emit(&app, events::PAGE_CHANGED, page.id.clone());
    Ok(page)
}

#[tauri::command(async)]
pub fn create_page(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> AppResult<SoundPage> {
    let page = pages::create(&state.db, &name)?;
    events::emit(&app, events::PAGE_CHANGED, page.id.clone());
    Ok(page)
}

#[tauri::command(async)]
pub fn rename_page(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
    name: String,
) -> AppResult<SoundPage> {
    let page = pages::rename(&state.db, &page_id, &name)?;
    events::emit(&app, events::PAGE_CHANGED, page.id.clone());
    Ok(page)
}

/// Cuantos slots ocupados tiene la pagina. El frontend lo consulta antes de
/// borrar para pedir confirmacion solo cuando hace falta (§8).
#[tauri::command(async)]
pub fn count_page_assignments(state: State<'_, AppState>, page_id: String) -> AppResult<i64> {
    pages::assigned_slot_count(&state.db, &page_id)
}

#[tauri::command(async)]
pub fn delete_page(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
) -> AppResult<Vec<PageSummary>> {
    pages::delete(&state.db, &page_id)?;

    if state.active_page().as_deref() == Some(page_id.as_str()) {
        state.set_active_page(None);
    }

    let summaries = pages::list_summaries(&state.db)?;
    events::emit(&app, events::PAGE_CHANGED, page_id);
    Ok(summaries)
}

#[tauri::command(async)]
pub fn reorder_pages(
    app: AppHandle,
    state: State<'_, AppState>,
    page_ids: Vec<String>,
) -> AppResult<Vec<PageSummary>> {
    let summaries = pages::reorder(&state.db, &page_ids)?;
    events::emit(&app, events::PAGE_CHANGED, String::new());
    Ok(summaries)
}

#[tauri::command(async)]
pub fn duplicate_page(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
) -> AppResult<SoundPage> {
    let page = pages::duplicate(&state.db, &page_id)?;
    events::emit(&app, events::PAGE_CHANGED, page.id.clone());
    Ok(page)
}

fn notify_slot(app: &AppHandle, page_id: &str, slot_number: u8) {
    events::emit(
        app,
        events::SLOT_CHANGED,
        SlotChangedPayload {
            page_id: page_id.to_string(),
            slot_number,
        },
    );
}

#[tauri::command(async)]
pub fn assign_sound_to_slot(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
    slot_number: SlotNumber,
    sound_id: String,
) -> AppResult<SoundSlot> {
    let slot = slots::assign(&state.db, &page_id, slot_number, &sound_id)?;
    notify_slot(&app, &page_id, slot_number.get());
    Ok(slot)
}

#[tauri::command(async)]
pub fn clear_slot(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
    slot_number: SlotNumber,
) -> AppResult<SoundSlot> {
    let slot = slots::clear(&state.db, &page_id, slot_number)?;
    notify_slot(&app, &page_id, slot_number.get());
    Ok(slot)
}

/// Mueve o intercambia el contenido de dos slots.
#[tauri::command(async)]
pub fn swap_slots(
    app: AppHandle,
    state: State<'_, AppState>,
    from_page_id: String,
    from_slot: SlotNumber,
    to_page_id: String,
    to_slot: SlotNumber,
) -> AppResult<Vec<SoundSlot>> {
    slots::swap(&state.db, &from_page_id, from_slot, &to_page_id, to_slot)?;

    notify_slot(&app, &from_page_id, from_slot.get());
    notify_slot(&app, &to_page_id, to_slot.get());

    let mut updated = Vec::new();
    if let Some(slot) = slots::get(&state.db, &from_page_id, from_slot)? {
        updated.push(slot);
    }
    if let Some(slot) = slots::get(&state.db, &to_page_id, to_slot)? {
        updated.push(slot);
    }
    Ok(updated)
}

#[tauri::command(async)]
pub fn set_slot_label(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
    slot_number: SlotNumber,
    label: Option<String>,
) -> AppResult<SoundSlot> {
    let slot = slots::set_label(&state.db, &page_id, slot_number, label.as_deref())?;
    notify_slot(&app, &page_id, slot_number.get());
    Ok(slot)
}

#[tauri::command(async)]
pub fn set_slot_volume(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
    slot_number: SlotNumber,
    volume: Option<f32>,
) -> AppResult<SoundSlot> {
    let slot = slots::set_volume(&state.db, &page_id, slot_number, volume)?;
    notify_slot(&app, &page_id, slot_number.get());
    Ok(slot)
}
