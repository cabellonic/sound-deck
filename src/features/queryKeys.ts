import type { LibraryFilter, SoundSortOrder } from '@/types/domain';

/** Claves de TanStack Query centralizadas para poder invalidar con precision. */
export const queryKeys = {
  appState: ['app-state'] as const,
  settings: ['settings'] as const,
  pages: ['pages'] as const,
  page: (pageId: string | undefined) => ['page', pageId ?? 'active'] as const,
  sounds: (text: string, filter: LibraryFilter, sort: SoundSortOrder) =>
    ['sounds', text, filter, sort] as const,
  facets: ['library-facets'] as const,
  storage: ['library-storage'] as const,
  folders: ['app-folders'] as const,
  devices: ['audio-devices'] as const,
  providers: ['providers'] as const,
  shortcutActions: ['shortcut-actions'] as const,
  remoteSearch: (query: string) => ['remote-search', query] as const,
  soundUsage: (soundId: string) => ['sound-usage', soundId] as const,
  imageExtensions: ['image-extensions'] as const,
};
