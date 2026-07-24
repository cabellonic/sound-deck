import {
  SLOT_NUMBERS,
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
