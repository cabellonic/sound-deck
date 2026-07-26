import {
  SLOT_NUMBERS,
  type AppSettings,
  type PageSummary,
  type RemoteSound,
  type Sound,
  type SoundPage,
  type SoundSlot,
} from '@/types/domain';

export function makeSound(overrides: Partial<Sound> = {}): Sound {
  return {
    id: 'sound-1',
    name: 'Bruh',
    originalName: 'bruh.mp3',
    fileExtension: 'mp3',
    fileSizeBytes: 48213,
    durationMs: 1234,
    source: { type: 'local_import' },
    providerCategory: null,
    normalizedCategory: 'uncategorized',
    tags: [],
    license: null,
    attribution: null,
    sourcePageUrl: null,
    // Por defecto un audio sigue el volumen general y no tiene imagen.
    customVolume: null,
    imagePath: null,
    playCount: 0,
    lastPlayedAt: null,
    createdAt: '2026-01-01T00:00:00Z',
    fileAvailable: true,
    assignedSlotCount: 0,
    assignedSlot: null,
    loudnessLufs: null,
    ...overrides,
  };
}

export function makeSlot(overrides: Partial<SoundSlot> = {}): SoundSlot {
  return {
    pageId: 'page-1',
    slotNumber: 1,
    sound: null,
    customLabel: null,
    customVolume: null,
    ...overrides,
  };
}

export function makePage(overrides: Partial<SoundPage> = {}): SoundPage {
  return {
    id: 'page-1',
    name: 'Principal',
    position: 0,
    slots: SLOT_NUMBERS.map((slotNumber) => makeSlot({ slotNumber })),
    ...overrides,
  };
}

export function makePageSummary(overrides: Partial<PageSummary> = {}): PageSummary {
  return {
    id: 'page-1',
    name: 'Principal',
    position: 0,
    assignedSlots: 0,
    ...overrides,
  };
}

export function makeRemoteSound(overrides: Partial<RemoteSound> = {}): RemoteSound {
  return {
    providerId: 'freesound',
    remoteId: '573661',
    title: 'bruh sound effect',
    description: null,
    durationMs: 1234,
    previewUrl: 'https://cdn.freesound.org/previews/573/573661-hq.mp3',
    sourcePageUrl: 'https://freesound.org/people/demo/sounds/573661/',
    downloadReference: '573661',
    providerCategory: null,
    normalizedCategory: 'memes',
    tags: ['meme', 'voice'],
    license: { code: 'cc0-1.0', name: 'CC0 1.0', url: null },
    attribution: 'bruh por demo en Freesound',
    fileSizeBytes: 48213,
    ...overrides,
  };
}

/** Configuracion con los mismos valores predeterminados que devuelve Rust. */
export function makeSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    general: {
      startWithSystem: false,
      minimizeToTray: true,
      closeToTray: true,
      showNotifications: true,
      overlayOnActiveMonitor: true,
      overlayPosition: null,
      overlaySize: null,
      closeOverlayAfterPlay: true,
      closeOverlayOnBlur: true,
      rememberLastPage: true,
      theme: 'system',
      language: 'es',
      onboardingCompleted: true,
      lastPageId: null,
    },
    audio: {
      outputDeviceId: null,
      outputDeviceName: null,
      masterVolume: 0.35,
      previewVolume: 0.2,
      playbackMode: 'interrupt',
      restartSameSound: true,
      maxDownloadBytes: 25 * 1024 * 1024,
      normalizeVolume: false,
      targetLufs: -18,
    },
    shortcuts: {
      bindings: [
        { action: 'toggle_overlay', accelerator: 'Alt+Home', scope: 'global' },
        { action: 'stop_all', accelerator: 'Alt+End', scope: 'global' },
        { action: 'prev_page', accelerator: 'PageUp', scope: 'overlay' },
        { action: 'next_page', accelerator: 'PageDown', scope: 'overlay' },
      ],
      globalSlotPlayback: false,
      slotModifier: 'ctrl_alt',
      allowKeyRepeat: false,
    },
    library: { logLevel: 'info' },
    ...overrides,
  };
}
