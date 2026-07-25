/**
 * Estado de interfaz y estado efimero.
 *
 * Aca vive solo lo que no pertenece a la base de datos: avisos, pestana activa,
 * texto de busqueda, indicadores de reproduccion y progreso de descargas. Los
 * datos persistentes se consultan con TanStack Query, no se duplican aca.
 */
import { create } from 'zustand';

import type { NoticeLevel } from '@/lib/events';
import type { LibraryFilter, SoundSortOrder } from '@/types/domain';

export type LibraryTab = 'saved' | 'internet';

export type SettingsTab =
  'general' | 'audio' | 'shortcuts' | 'library' | 'providers' | 'advanced' | 'credits';

export interface Toast {
  id: string;
  level: NoticeLevel;
  message: string;
}

/** Estado de una descarga en curso, para la barra de progreso del resultado. */
export interface DownloadState {
  receivedBytes: number;
  totalBytes: number | null;
  status: 'downloading' | 'validating' | 'failed';
  message?: string;
}

interface UiState {
  toasts: Toast[];
  pushToast: (level: NoticeLevel, message: string) => void;
  dismissToast: (id: string) => void;

  libraryTab: LibraryTab;
  setLibraryTab: (tab: LibraryTab) => void;

  /**
   * Cada pestana recuerda su propia busqueda.
   *
   * Compartir el texto hacia que volver a "Guardados" desde "Internet" dejara
   * la biblioteca filtrada por lo ultimo que se busco online.
   */
  searchByTab: Record<LibraryTab, string>;
  setSearchText: (tab: LibraryTab, text: string) => void;

  libraryFilter: LibraryFilter;
  setLibraryFilter: (filter: LibraryFilter) => void;

  sortOrder: SoundSortOrder;
  setSortOrder: (sort: SoundSortOrder) => void;

  settingsOpen: boolean;
  setSettingsOpen: (open: boolean) => void;

  settingsTab: SettingsTab;
  setSettingsTab: (tab: SettingsTab) => void;
  /** Abre la configuracion directamente en la seccion indicada. */
  openSettings: (tab?: SettingsTab) => void;

  /** Ids de sonidos sonando ahora mismo. Lo alimentan los eventos de Rust. */
  playingSoundIds: string[];
  setPlayingSoundIds: (ids: string[]) => void;

  /** Preview sonando: `local:<id>` o `remote:<provider>:<id>`. */
  previewKey: string | null;
  setPreviewKey: (key: string | null) => void;

  /**
   * Preview pedida que todavia se esta bajando.
   *
   * Un resultado online hay que descargarlo antes de que suene. En ese rato no
   * suena nada, asi que la fila tiene que decir que esta cargando y no fingir
   * que ya esta reproduciendo.
   */
  previewLoadingKey: string | null;
  setPreviewLoadingKey: (key: string | null) => void;

  /** Descargas activas, indexadas por `<provider>:<remoteId>`. */
  downloads: Record<string, DownloadState>;
  setDownload: (key: string, state: DownloadState | null) => void;
}

let toastCounter = 0;

export const useUiStore = create<UiState>((set) => ({
  toasts: [],
  pushToast: (level, message) =>
    set((state) => {
      // Evitamos apilar el mismo aviso repetido (por ejemplo al reintentar).
      if (state.toasts.some((toast) => toast.message === message && toast.level === level)) {
        return state;
      }
      toastCounter += 1;
      const toast: Toast = { id: `toast-${toastCounter}`, level, message };
      // Como maximo cuatro avisos visibles a la vez.
      return { toasts: [...state.toasts, toast].slice(-4) };
    }),
  dismissToast: (id) =>
    set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),

  libraryTab: 'saved',
  setLibraryTab: (libraryTab) => set({ libraryTab }),

  searchByTab: { saved: '', internet: '' },
  setSearchText: (tab, text) =>
    set((state) => ({ searchByTab: { ...state.searchByTab, [tab]: text } })),

  libraryFilter: { type: 'all' },
  setLibraryFilter: (libraryFilter) => set({ libraryFilter }),

  sortOrder: 'relevance',
  setSortOrder: (sortOrder) => set({ sortOrder }),

  settingsOpen: false,
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),

  settingsTab: 'general',
  setSettingsTab: (settingsTab) => set({ settingsTab }),
  openSettings: (tab) =>
    set((state) => ({ settingsOpen: true, settingsTab: tab ?? state.settingsTab })),

  playingSoundIds: [],
  setPlayingSoundIds: (playingSoundIds) => set({ playingSoundIds }),

  previewKey: null,
  setPreviewKey: (previewKey) => set({ previewKey }),

  previewLoadingKey: null,
  setPreviewLoadingKey: (previewLoadingKey) => set({ previewLoadingKey }),

  downloads: {},
  setDownload: (key, state) =>
    set((current) => {
      const downloads = { ...current.downloads };
      if (state === null) {
        delete downloads[key];
      } else {
        downloads[key] = state;
      }
      return { downloads };
    }),
}));

/** Atajo para mostrar un aviso desde fuera de un componente. */
export function notify(level: NoticeLevel, message: string): void {
  useUiStore.getState().pushToast(level, message);
}
