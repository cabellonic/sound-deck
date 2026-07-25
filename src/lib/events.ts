/**
 * Eventos que llegan desde Rust (§24), con tipos.
 *
 * El frontend nunca hace polling de estos estados: se suscribe y reacciona.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { AppSettings } from '@/types/domain';

export interface PlaybackPayload {
  soundId: string | null;
  isPreview: boolean;
}

export interface PlaybackErrorPayload {
  soundId: string | null;
  message: string;
}

export interface DownloadProgressPayload {
  operationId: string;
  providerId: string;
  remoteId: string;
  receivedBytes: number;
  totalBytes: number | null;
}

export interface DownloadCompletedPayload {
  operationId: string;
  providerId: string;
  remoteId: string;
  soundId: string;
  deduplicated: boolean;
}

export interface DownloadFailedPayload {
  operationId: string;
  providerId: string;
  remoteId: string;
  message: string;
}

export interface DeviceChangedPayload {
  deviceName: string;
  deviceId: string | null;
  notice: string | null;
}

export interface SlotChangedPayload {
  pageId: string;
  slotNumber: number;
}

export interface ShortcutTriggeredPayload {
  action: string;
}

export interface OverlayVisibilityPayload {
  visible: boolean;
}

export type NoticeLevel = 'info' | 'success' | 'warning' | 'error';

export interface NoticePayload {
  level: NoticeLevel;
  message: string;
}

/** Mapa evento -> payload. Mantener sincronizado con `events/mod.rs`. */
export interface AppEventMap {
  'playback-started': PlaybackPayload;
  'playback-stopped': PlaybackPayload;
  'playback-error': PlaybackErrorPayload;
  'download-progress': DownloadProgressPayload;
  'download-completed': DownloadCompletedPayload;
  'download-failed': DownloadFailedPayload;
  'audio-device-changed': DeviceChangedPayload;
  'audio-device-lost': DeviceChangedPayload;
  'page-changed': string;
  'slot-changed': SlotChangedPayload;
  'library-changed': null;
  'shortcut-triggered': ShortcutTriggeredPayload;
  'overlay-visibility-changed': OverlayVisibilityPayload;
  'overlay-placement-changed': { placing: boolean };
  'settings-changed': AppSettings;
  notice: NoticePayload;
  'open-settings': null;
}

export type AppEventName = keyof AppEventMap;

/** Suscribe a un evento tipado. Devuelve la funcion para desuscribirse. */
export function onAppEvent<K extends AppEventName>(
  event: K,
  handler: (payload: AppEventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<AppEventMap[K]>(event, ({ payload }) => handler(payload));
}
