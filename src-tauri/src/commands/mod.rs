//! Comandos expuestos al frontend.
//!
//! Cada comando es pequeno y con una responsabilidad clara (§23). Los que hacen
//! trabajo bloqueante estan marcados con `#[tauri::command(async)]` o mueven el
//! trabajo a `spawn_blocking`, para no ocupar el hilo principal.

pub mod audio_devices;
pub mod library;
pub mod pages;
pub mod playback;
pub mod providers;
pub mod settings;
pub mod window;

/// Registra todos los comandos en el builder de Tauri.
///
/// Es una macro y no una funcion porque `generate_handler!` necesita la lista
/// literal de comandos en tiempo de compilacion.
#[macro_export]
macro_rules! command_handlers {
    () => {
        tauri::generate_handler![
            // Estado general y ventanas
            $crate::commands::window::get_app_state,
            $crate::commands::window::show_overlay,
            $crate::commands::window::hide_overlay,
            $crate::commands::window::toggle_overlay,
            $crate::commands::window::focus_main_window,
            $crate::commands::window::complete_onboarding,
            // Paginas y slots
            $crate::commands::pages::list_pages,
            $crate::commands::pages::get_page,
            $crate::commands::pages::set_active_page,
            $crate::commands::pages::create_page,
            $crate::commands::pages::rename_page,
            $crate::commands::pages::delete_page,
            $crate::commands::pages::count_page_assignments,
            $crate::commands::pages::reorder_pages,
            $crate::commands::pages::duplicate_page,
            $crate::commands::pages::assign_sound_to_slot,
            $crate::commands::pages::clear_slot,
            $crate::commands::pages::swap_slots,
            $crate::commands::pages::set_slot_label,
            $crate::commands::pages::set_slot_volume,
            // Biblioteca local
            $crate::commands::library::search_local_sounds,
            $crate::commands::library::get_library_facets,
            $crate::commands::library::import_sound_files,
            $crate::commands::library::rename_sound,
            $crate::commands::library::update_sound_volume,
            $crate::commands::library::update_sound_tags,
            $crate::commands::library::set_sound_image,
            $crate::commands::library::clear_sound_image,
            $crate::commands::library::supported_image_extensions,
            $crate::commands::library::get_sound_usage,
            $crate::commands::library::delete_sound,
            $crate::commands::library::get_library_storage,
            $crate::commands::library::find_missing_sounds,
            $crate::commands::library::remove_orphan_sounds,
            $crate::commands::library::clean_temp_files,
            $crate::commands::library::backup_database,
            $crate::commands::library::supported_audio_extensions,
            $crate::commands::library::get_app_folders,
            $crate::commands::library::reveal_sound_in_folder,
            // Reproduccion
            $crate::commands::playback::play_sound,
            $crate::commands::playback::play_slot,
            $crate::commands::playback::preview_local_sound,
            $crate::commands::playback::preview_remote_sound,
            $crate::commands::playback::stop_preview,
            $crate::commands::playback::stop_all,
            $crate::commands::playback::get_playback_status,
            // Dispositivos de audio
            $crate::commands::audio_devices::list_audio_devices,
            $crate::commands::audio_devices::select_audio_device,
            $crate::commands::audio_devices::use_default_audio_device,
            $crate::commands::audio_devices::test_audio_device,
            // Configuracion y atajos
            $crate::commands::settings::get_settings,
            $crate::commands::settings::update_settings,
            $crate::commands::settings::reset_settings,
            $crate::commands::settings::register_shortcut,
            $crate::commands::settings::reset_shortcuts,
            $crate::commands::settings::list_shortcut_actions,
            $crate::commands::settings::set_autostart,
            // Proveedores online
            $crate::commands::providers::list_providers,
            $crate::commands::providers::set_provider_enabled,
            $crate::commands::providers::set_provider_api_key,
            $crate::commands::providers::test_provider_connection,
            $crate::commands::providers::search_remote_sounds,
            $crate::commands::providers::download_remote_sound,
            $crate::commands::providers::download_and_assign_remote_sound,
        ]
    };
}
