/**
 * Capa tipada sobre `invoke`.
 *
 * Cada comando del backend tiene aca su firma. Es el unico lugar del frontend
 * donde aparece un string de comando, asi que un rename en Rust rompe la
 * compilacion de TypeScript en un solo punto.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

import type {
  AppFolders,
  AppSettings,
  AppStateSnapshot,
  AudioDeviceInfo,
  AudioDeviceList,
  ImportReport,
  LibraryFacets,
  LibraryStorage,
  PageSummary,
  PlaybackStatus,
  ProviderSearchResult,
  ProviderStatus,
  SettingsPatch,
  ShortcutAction,
  ShortcutActionInfo,
  ShortcutUpdate,
  SlotNumber,
  Sound,
  SoundPage,
  SoundQuery,
  SoundSlot,
  SoundUsage,
  AppError,
} from '@/types/domain';

/** Error de dominio ya normalizado, listo para mostrar. */
export class IpcError extends Error {
  readonly code: string;
  readonly recoverable: boolean;
  readonly details: Record<string, string>;

  constructor(error: AppError) {
    super(error.message);
    this.name = 'IpcError';
    this.code = error.code;
    this.recoverable = error.recoverable;
    this.details = error.details ?? {};
  }
}

/**
 * Convierte lo que rechaza `invoke` en un `IpcError`.
 *
 * El backend siempre devuelve un `AppError`, pero un fallo del propio puente
 * IPC (ventana cerrandose, comando inexistente) llega como string. Es el unico
 * borde donde aceptamos `unknown` (§4.4).
 */
function normalizeError(error: unknown): IpcError {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error &&
    typeof (error as AppError).message === 'string'
  ) {
    return new IpcError(error as AppError);
  }

  return new IpcError({
    code: 'UNKNOWN',
    message:
      typeof error === 'string' && error.length > 0
        ? error
        : 'Ocurrio un error inesperado. Revisa los logs para mas detalle.',
    recoverable: true,
  });
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

/** Mensaje legible de cualquier error atrapado en la interfaz. */
export function errorMessage(error: unknown): string {
  if (error instanceof IpcError) return error.message;
  if (error instanceof Error) return error.message;
  return 'Ocurrio un error inesperado.';
}

/** Detalle estructurado que el backend adjunta para ofrecer acciones. */
export function errorDetail(error: unknown, key: string): string | undefined {
  return error instanceof IpcError ? error.details[key] : undefined;
}

// --- Estado general y ventanas ---------------------------------------------

export const getAppState = () => call<AppStateSnapshot>('get_app_state');
export const showOverlay = () => call<void>('show_overlay');
export const hideOverlay = () => call<void>('hide_overlay');
export const toggleOverlay = () => call<void>('toggle_overlay');
export const focusMainWindow = () => call<void>('focus_main_window');
export const completeOnboarding = () => call<void>('complete_onboarding');

// --- Paginas y slots --------------------------------------------------------

export const listPages = () => call<PageSummary[]>('list_pages');
export const getPage = (pageId?: string) => call<SoundPage>('get_page', { pageId });
export const setActivePage = (pageId: string) => call<SoundPage>('set_active_page', { pageId });
export const createPage = (name: string) => call<SoundPage>('create_page', { name });
export const renamePage = (pageId: string, name: string) =>
  call<SoundPage>('rename_page', { pageId, name });
export const deletePage = (pageId: string) => call<PageSummary[]>('delete_page', { pageId });
export const countPageAssignments = (pageId: string) =>
  call<number>('count_page_assignments', { pageId });
export const reorderPages = (pageIds: string[]) =>
  call<PageSummary[]>('reorder_pages', { pageIds });
export const duplicatePage = (pageId: string) => call<SoundPage>('duplicate_page', { pageId });

export const assignSoundToSlot = (pageId: string, slotNumber: SlotNumber, soundId: string) =>
  call<SoundSlot>('assign_sound_to_slot', { pageId, slotNumber, soundId });
export const clearSlot = (pageId: string, slotNumber: SlotNumber) =>
  call<SoundSlot>('clear_slot', { pageId, slotNumber });
export const swapSlots = (
  fromPageId: string,
  fromSlot: SlotNumber,
  toPageId: string,
  toSlot: SlotNumber,
) => call<SoundSlot[]>('swap_slots', { fromPageId, fromSlot, toPageId, toSlot });
export const setSlotLabel = (pageId: string, slotNumber: SlotNumber, label: string | null) =>
  call<SoundSlot>('set_slot_label', { pageId, slotNumber, label });
export const setSlotVolume = (pageId: string, slotNumber: SlotNumber, volume: number | null) =>
  call<SoundSlot>('set_slot_volume', { pageId, slotNumber, volume });

// --- Biblioteca local -------------------------------------------------------

export const searchLocalSounds = (query: Partial<SoundQuery>) =>
  call<Sound[]>('search_local_sounds', { query });
export const getLibraryFacets = () => call<LibraryFacets>('get_library_facets');
export const importSoundFiles = (paths: string[]) =>
  call<ImportReport>('import_sound_files', { paths });
export const renameSound = (soundId: string, name: string) =>
  call<Sound>('rename_sound', { soundId, name });
/** `null` vuelve a linkear el audio al volumen general. */
export const updateSoundVolume = (soundId: string, volume: number | null) =>
  call<Sound>('update_sound_volume', { soundId, volume });
export const updateSoundTags = (soundId: string, tags: string[]) =>
  call<Sound>('update_sound_tags', { soundId, tags });
export const setSoundImage = (soundId: string, path: string) =>
  call<Sound>('set_sound_image', { soundId, path });
export const clearSoundImage = (soundId: string) => call<Sound>('clear_sound_image', { soundId });
export const supportedImageExtensions = () => call<string[]>('supported_image_extensions');

/**
 * URL que el WebView puede cargar para la imagen de un audio.
 *
 * Es el unico lugar del frontend que toca `imagePath`. El protocolo `asset:`
 * necesita la ruta de disco, y el `assetProtocol.scope` de `tauri.conf.json`
 * la acota a la carpeta de imagenes: aunque la ruta viaje por IPC, no habilita
 * leer nada mas del disco.
 */
export function soundImageSrc(sound: Pick<Sound, 'imagePath'> | null | undefined): string | null {
  return sound?.imagePath ? convertFileSrc(sound.imagePath) : null;
}
export const getSoundUsage = (soundId: string) =>
  call<SoundUsage[]>('get_sound_usage', { soundId });
export const deleteSound = (soundId: string) => call<void>('delete_sound', { soundId });
export const getLibraryStorage = () => call<LibraryStorage>('get_library_storage');
export const findMissingSounds = () => call<Sound[]>('find_missing_sounds');
export const removeOrphanSounds = () => call<number>('remove_orphan_sounds');
export const cleanTempFiles = () => call<number>('clean_temp_files');
export const backupDatabase = () => call<string>('backup_database');
export const supportedAudioExtensions = () => call<string[]>('supported_audio_extensions');
export const getAppFolders = () => call<AppFolders>('get_app_folders');
export const revealSoundInFolder = (soundId: string) =>
  call<string>('reveal_sound_in_folder', { soundId });

// --- Reproduccion -----------------------------------------------------------

export const playSound = (soundId: string) => call<void>('play_sound', { soundId });
export const playSlot = (pageId: string, slotNumber: SlotNumber) =>
  call<void>('play_slot', { pageId, slotNumber });
export const previewLocalSound = (soundId: string) =>
  call<void>('preview_local_sound', { soundId });
export const previewRemoteSound = (providerId: string, remoteId: string) =>
  call<void>('preview_remote_sound', { providerId, remoteId });
export const stopPreview = () => call<void>('stop_preview');
export const stopAll = () => call<void>('stop_all');
export const getPlaybackStatus = () => call<PlaybackStatus>('get_playback_status');

// --- Dispositivos de audio --------------------------------------------------

export const listAudioDevices = () => call<AudioDeviceList>('list_audio_devices');
export const selectAudioDevice = (deviceId: string | null, deviceName: string | null) =>
  call<AudioDeviceInfo>('select_audio_device', { deviceId, deviceName });
export const useDefaultAudioDevice = () => call<AudioDeviceInfo>('use_default_audio_device');
export const testAudioDevice = () => call<void>('test_audio_device');

// --- Configuracion y atajos -------------------------------------------------

export const getSettings = () => call<AppSettings>('get_settings');
export const updateSettings = (patch: SettingsPatch) =>
  call<AppSettings>('update_settings', { patch });
export const resetSettings = () => call<AppSettings>('reset_settings');
export const registerShortcut = (action: ShortcutAction, accelerator: string) =>
  call<ShortcutUpdate>('register_shortcut', { action, accelerator });
export const resetShortcuts = () => call<ShortcutUpdate>('reset_shortcuts');
export const listShortcutActions = () => call<ShortcutActionInfo[]>('list_shortcut_actions');
export const setAutostart = (enabled: boolean) => call<boolean>('set_autostart', { enabled });

// --- Proveedores online -----------------------------------------------------

export const listProviders = () => call<ProviderStatus[]>('list_providers');
export const setProviderEnabled = (providerId: string, enabled: boolean) =>
  call<ProviderStatus[]>('set_provider_enabled', { providerId, enabled });
export const setProviderApiKey = (providerId: string, apiKey: string | null) =>
  call<ProviderStatus[]>('set_provider_api_key', { providerId, apiKey });
export const testProviderConnection = (providerId: string) =>
  call<void>('test_provider_connection', { providerId });
export const searchRemoteSounds = (query: string, page?: number, pageSize?: number) =>
  call<ProviderSearchResult[]>('search_remote_sounds', { query, page, pageSize });
export const downloadRemoteSound = (providerId: string, remoteId: string) =>
  call<Sound>('download_remote_sound', { providerId, remoteId });
export const downloadAndAssignRemoteSound = (
  providerId: string,
  remoteId: string,
  pageId: string,
  slotNumber: SlotNumber,
) =>
  call<SoundSlot>('download_and_assign_remote_sound', { providerId, remoteId, pageId, slotNumber });
