/**
 * Tipos de dominio compartidos con el backend.
 *
 * Son el reflejo exacto de los structs de Rust en `src-tauri/src/domain`.
 * Cualquier cambio alla debe replicarse aca; los tests de contrato en
 * `src/tests/contract.test.ts` verifican que las constantes coincidan.
 */

export type NormalizedCategory =
  | 'memes'
  | 'reactions'
  | 'games'
  | 'anime'
  | 'movies_tv'
  | 'music'
  | 'sound_effects'
  | 'voices'
  | 'sports'
  | 'other'
  | 'uncategorized';

export const NORMALIZED_CATEGORIES: readonly NormalizedCategory[] = [
  'memes',
  'reactions',
  'games',
  'anime',
  'movies_tv',
  'music',
  'sound_effects',
  'voices',
  'sports',
  'other',
  'uncategorized',
] as const;

export type SoundSource =
  { type: 'local_import' } | { type: 'provider'; providerId: string; remoteId: string };

export interface SoundLicense {
  code: string;
  name: string;
  url: string | null;
}

export interface Sound {
  id: string;
  name: string;
  originalName: string | null;
  fileExtension: string | null;
  fileSizeBytes: number | null;
  durationMs: number | null;
  source: SoundSource;
  providerCategory: string | null;
  normalizedCategory: NormalizedCategory;
  tags: string[];
  license: SoundLicense | null;
  attribution: string | null;
  sourcePageUrl: string | null;
  /** Volumen absoluto propio. `null` = linkeado al volumen general. */
  customVolume: number | null;
  /**
   * Ruta absoluta de la imagen administrada, o `null` si no tiene.
   *
   * Es la unica ruta de disco que cruza el IPC, y cruza porque el protocolo
   * `asset:` necesita una ruta para mostrarla. Usala siempre a traves de
   * `soundImageSrc()`, nunca la muestres al usuario.
   */
  imagePath: string | null;
  playCount: number;
  lastPlayedAt: string | null;
  createdAt: string;
  /** `false` cuando el archivo administrado ya no esta en disco. */
  fileAvailable: boolean;
  assignedSlotCount: number;
  /**
   * El unico boton que lo usa, cuando `assignedSlotCount === 1`.
   *
   * Solo lo completa la busqueda de la biblioteca, que es donde se muestra; un
   * sonido que llega dentro de un slot no lo trae.
   */
  assignedSlot: SoundUsage | null;
  /** Sonoridad medida en LUFS. `null` si todavia no se midio. */
  loudnessLufs: number | null;
}

export type SlotNumber = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

export const SLOT_NUMBERS: readonly SlotNumber[] = [1, 2, 3, 4, 5, 6, 7, 8, 9] as const;
export const SLOTS_PER_PAGE = 9;
export const MAX_PAGES = 9;

export interface SoundSlot {
  pageId: string;
  slotNumber: SlotNumber;
  sound: Sound | null;
  customLabel: string | null;
  /** Volumen absoluto del boton. `null` = usa el del audio. */
  customVolume: number | null;
}

export interface SoundPage {
  id: string;
  name: string;
  position: number;
  slots: SoundSlot[];
}

export interface PageSummary {
  id: string;
  name: string;
  position: number;
  assignedSlots: number;
}

export type PlaybackMode = 'interrupt' | 'overlap';
export type ThemePreference = 'system' | 'dark' | 'light';

export interface GeneralSettings {
  startWithSystem: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  showNotifications: boolean;
  overlayOnActiveMonitor: boolean;
  /** Posicion elegida a mano, en pixeles fisicos. `null` centra automaticamente. */
  overlayPosition: { x: number; y: number } | null;
  /** Tamano elegido a mano, en pixeles logicos. `null` usa el de la ventana original. */
  overlaySize: { width: number; height: number } | null;
  closeOverlayAfterPlay: boolean;
  closeOverlayOnBlur: boolean;
  rememberLastPage: boolean;
  theme: ThemePreference;
  language: string;
  onboardingCompleted: boolean;
  lastPageId: string | null;
}

export interface AudioSettings {
  outputDeviceId: string | null;
  outputDeviceName: string | null;
  masterVolume: number;
  previewVolume: number;
  playbackMode: PlaybackMode;
  restartSameSound: boolean;
  maxDownloadBytes: number;
  /** Igualar el volumen entre audios con la sonoridad medida. */
  normalizeVolume: boolean;
  /** Sonoridad objetivo en LUFS cuando la normalizacion esta activa. */
  targetLufs: number;
}

export type ShortcutAction = 'toggle_overlay' | 'stop_all' | 'prev_page' | 'next_page';
export type ShortcutScope = 'global' | 'overlay';

export interface ShortcutBinding {
  action: ShortcutAction;
  accelerator: string;
  scope: ShortcutScope;
}

export type SlotModifier = 'ctrl_alt' | 'ctrl_shift' | 'alt_shift';

export const SLOT_MODIFIERS: readonly SlotModifier[] = ['ctrl_alt', 'ctrl_shift', 'alt_shift'];

/** Prefijo en el formato de Tauri, espejo de `SlotModifier::prefix` en Rust. */
export const SLOT_MODIFIER_PREFIX: Record<SlotModifier, string> = {
  ctrl_alt: 'Ctrl+Alt',
  ctrl_shift: 'Ctrl+Shift',
  alt_shift: 'Alt+Shift',
};

export interface ShortcutSettings {
  bindings: ShortcutBinding[];
  globalSlotPlayback: boolean;
  slotModifier: SlotModifier;
  allowKeyRepeat: boolean;
}

export interface LibrarySettings {
  logLevel: string;
}

export interface AppSettings {
  general: GeneralSettings;
  audio: AudioSettings;
  shortcuts: ShortcutSettings;
  library: LibrarySettings;
}

/** Parche parcial: solo se envian las secciones que cambian. */
export interface SettingsPatch {
  general?: GeneralSettings;
  audio?: AudioSettings;
  shortcuts?: ShortcutSettings;
  library?: LibrarySettings;
}

export type SoundSortOrder = 'relevance' | 'recent' | 'most_played' | 'name';

export type LibraryFilter =
  | { type: 'all' }
  | { type: 'recent' }
  | { type: 'most_played' }
  | { type: 'unassigned' }
  | { type: 'uncategorized' }
  | { type: 'category'; category: NormalizedCategory }
  | { type: 'provider'; providerId: string };

export interface SoundQuery {
  text: string;
  filter: LibraryFilter;
  sort: SoundSortOrder;
  limit: number;
  offset: number;
}

export interface FacetCount {
  value: string;
  count: number;
}

export interface LibraryFacets {
  total: number;
  unassigned: number;
  categories: FacetCount[];
  providers: FacetCount[];
}

export interface SoundUsage {
  pageId: string;
  pageName: string;
  slotNumber: number;
}

export interface ImportFailure {
  fileName: string;
  message: string;
}

export interface ImportReport {
  imported: Sound[];
  duplicates: Sound[];
  failed: ImportFailure[];
}

export interface AudioDeviceInfo {
  id: string | null;
  name: string;
  isDefault: boolean;
}

export interface AudioDeviceList {
  devices: AudioDeviceInfo[];
  current: AudioDeviceInfo | null;
}

export interface RemoteSound {
  providerId: string;
  remoteId: string;
  title: string;
  description: string | null;
  durationMs: number | null;
  previewUrl: string | null;
  sourcePageUrl: string | null;
  downloadReference: string | null;
  providerCategory: string | null;
  normalizedCategory: NormalizedCategory | null;
  tags: string[];
  license: SoundLicense | null;
  attribution: string | null;
  fileSizeBytes: number | null;
}

export interface ProviderSearchResult {
  providerId: string;
  providerName: string;
  items: RemoteSound[];
  hasMore: boolean;
  total: number | null;
  /** Mensaje si este proveedor concreto fallo; los demas siguen mostrando resultados. */
  error: string | null;
}

export interface ProviderStatus {
  id: string;
  displayName: string;
  homepage: string;
  enabled: boolean;
  requiresApiKey: boolean;
  hasApiKey: boolean;
  maskedApiKey: string | null;
  unofficial: boolean;
  supportsPreview: boolean;
  supportsDownload: boolean;
  ready: boolean;
  /** Si puede conectar una cuenta por OAuth2 para bajar el archivo original. */
  supportsOauth: boolean;
  hasClientId: boolean;
  accountConnected: boolean;
}

export interface LibraryStorage {
  soundsDir: string;
  usedBytes: number;
  usedReadable: string;
  missingFiles: number;
}

export interface AppFolders {
  sounds: string;
  logs: string;
  data: string;
}

export interface PlaybackStatus {
  playingSoundIds: string[];
  previewing: boolean;
}

export interface ShortcutConflict {
  accelerator: string;
  actions: string[];
}

export interface RejectedShortcut {
  action: string;
  accelerator: string;
  message: string;
}

export interface RegistrationReport {
  registered: string[];
  rejected: RejectedShortcut[];
}

export interface ShortcutUpdate {
  settings: ShortcutSettings;
  conflicts: ShortcutConflict[];
  registration: RegistrationReport;
  applied: string;
}

export interface ShortcutActionInfo {
  action: ShortcutAction;
  label: string;
  scope: ShortcutScope;
}

export interface AppStateSnapshot {
  settings: AppSettings;
  pages: PageSummary[];
  activePage: SoundPage | null;
  providers: ProviderStatus[];
  audioDevice: AudioDeviceInfo | null;
  version: string;
  supportsFocusRestore: boolean;
  overlayVisible: boolean;
}

/** Error de dominio serializado por el backend (§29). */
export interface AppError {
  code: string;
  message: string;
  recoverable: boolean;
  details?: Record<string, string>;
}
