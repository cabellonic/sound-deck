import {
  AlertTriangle,
  ChevronDown,
  Globe,
  Loader2,
  Search,
  Settings2,
  Upload,
  X,
} from 'lucide-react';
import { useMemo } from 'react';

import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/primitives';
import { useLibraryFacets, useLibraryMutations, useLocalSounds } from '@/features/useLibrary';
import { hasReadyProvider, useProviders, useRemoteSearch } from '@/features/useRemoteSearch';
import { usePlaybackActions } from '@/features/useSoundboard';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import { useUiStore, type SettingsTab } from '@/stores/useUiStore';
import type { RemoteSound, Sound } from '@/types/domain';

import { FilterBar } from './FilterBar';
import { RemoteRow, type RemoteItemStatus } from './RemoteRow';
import { SoundRow } from './SoundRow';
import { VirtualGrid } from './VirtualGrid';

export interface LibraryPanelProps {
  onAssignLocal: (sound: Sound) => void;
  onAssignRemote: (item: RemoteSound) => void;
  onRenameSound: (sound: Sound) => void;
  onEditSoundVolume: (sound: Sound) => void;
  onDeleteSound: (sound: Sound) => void;
  onRevealSound: (sound: Sound) => void;
  onOpenUrl: (url: string) => void;
  /** Abre la configuracion, opcionalmente en una seccion concreta. */
  onOpenSettings: (tab?: SettingsTab) => void;
  /** `true` mientras hay archivos del sistema sobrevolando la ventana. */
  fileDropActive: boolean;
  /** Fila que va a recibir la imagen que se esta arrastrando, si hay alguna. */
  imageDropKey: string | null;
}

function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  spinning = false,
}: {
  icon: typeof Search;
  title: string;
  description: string;
  action?: React.ReactNode;
  /** Gira el icono. Sin esto un spinner queda quieto y parece que se colgo. */
  spinning?: boolean;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-6 py-12 text-center">
      <Icon className={cn('h-6 w-6 text-fg-subtle', spinning && 'animate-spin')} aria-hidden />
      <p className="text-sm font-medium text-fg-default">{title}</p>
      <p className="max-w-xs text-xs leading-relaxed text-fg-subtle">{description}</p>
      {action}
    </div>
  );
}

export function LibraryPanel(props: LibraryPanelProps) {
  const { t } = useTranslation();
  const tab = useUiStore((state) => state.libraryTab);
  const setTab = useUiStore((state) => state.setLibraryTab);
  // Cada pestana conserva su propia busqueda al ir y volver.
  const searchText = useUiStore((state) => state.searchByTab[state.libraryTab]);
  const setSearchText = useUiStore((state) => state.setSearchText);
  const filter = useUiStore((state) => state.libraryFilter);
  const setFilter = useUiStore((state) => state.setLibraryFilter);
  const sort = useUiStore((state) => state.sortOrder);
  const setSort = useUiStore((state) => state.setSortOrder);
  const previewKey = useUiStore((state) => state.previewKey);
  const previewLoadingKey = useUiStore((state) => state.previewLoadingKey);
  const playingSoundIds = useUiStore((state) => state.playingSoundIds);
  const downloads = useUiStore((state) => state.downloads);

  const sounds = useLocalSounds();
  const facets = useLibraryFacets();
  const providers = useProviders();
  const playback = usePlaybackActions();
  const { importWithDialog, download, pickImageWithDialog, clearImage } = useLibraryMutations();

  const providersReady = hasReadyProvider(providers.data);
  const remote = useRemoteSearch(searchText, tab === 'internet' && providersReady);

  // Guarda el id local ademas de la clave remota: con el se le puede poner una
  // imagen a un resultado que ya se descargo, sin volver a bajarlo.
  const savedIds = useMemo(() => {
    const byRemoteKey = new Map<string, string>();
    for (const sound of sounds.data ?? []) {
      if (sound.source.type === 'provider') {
        byRemoteKey.set(`${sound.source.providerId}:${sound.source.remoteId}`, sound.id);
      }
    }
    return byRemoteKey;
  }, [sounds.data]);

  const remoteStatus = (item: RemoteSound): RemoteItemStatus => {
    const key = `${item.providerId}:${item.remoteId}`;
    if (downloads[key]?.status === 'downloading') return 'downloading';
    if (downloads[key]?.status === 'failed') return 'error';
    if (savedIds.has(key)) return 'saved';
    // Cargando antes que guardado: mientras se baja la preview es lo unico que
    // le importa al usuario de esa fila.
    if (previewLoadingKey === `remote:${key}`) return 'loading-preview';
    if (previewKey === `remote:${key}`) return 'previewing';
    if (!item.previewUrl) return 'unavailable';
    return 'idle';
  };

  // Un mismo audio puede repetirse entre paginas si el proveedor reordena;
  // deduplicamos para no montar dos filas con la misma clave.
  const remoteItems = useMemo(() => {
    const seen = new Set<string>();
    const items: RemoteSound[] = [];
    for (const page of remote.data?.pages ?? []) {
      for (const result of page) {
        for (const item of result.items) {
          const key = `${item.providerId}:${item.remoteId}`;
          if (seen.has(key)) continue;
          seen.add(key);
          items.push(item);
        }
      }
    }
    return items;
  }, [remote.data]);

  // Los errores del ultimo intento alcanzan: repetirlos por pagina seria ruido.
  const lastRemotePage = remote.data?.pages.at(-1) ?? [];
  const remoteErrors = lastRemotePage.filter((result) => result.error !== null);

  return (
    <section
      // Soltar archivos del sistema aca los importa. El drop lo maneja Tauri a
      // nivel de ventana (§10) y resuelve el destino por posicion.
      data-file-drop-zone="library"
      className={cn(
        'flex min-h-0 flex-1 flex-col border-l border-border-subtle bg-surface-0',
        props.fileDropActive && 'ring-2 ring-inset ring-accent',
      )}
      aria-label={t('library.title')}
    >
      <Tabs
        value={tab}
        onValueChange={(value) => setTab(value as 'saved' | 'internet')}
        className="flex min-h-0 flex-1 flex-col"
      >
        <div className="flex flex-col gap-2.5 border-b border-border-subtle p-3">
          <div className="flex items-center gap-2">
            <div className="relative flex-1">
              <Search
                className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-fg-subtle"
                aria-hidden
              />
              <Input
                value={searchText}
                onChange={(event) => setSearchText(tab, event.target.value)}
                placeholder={tab === 'saved' ? t('library.searchSaved') : t('library.searchOnline')}
                aria-label={
                  tab === 'saved' ? t('library.searchSavedLabel') : t('library.searchOnlineLabel')
                }
                className="pl-8 pr-8"
              />
              {searchText ? (
                <button
                  type="button"
                  onClick={() => setSearchText(tab, '')}
                  aria-label={t('library.clearSearch')}
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-fg-subtle hover:text-fg-default"
                >
                  <X className="h-3.5 w-3.5" aria-hidden />
                </button>
              ) : null}
            </div>

            <Button onClick={() => void importWithDialog()} size="md">
              <Upload className="h-4 w-4" aria-hidden />
              {t('common.import')}
            </Button>
          </div>

          <div className="flex items-center gap-2">
            <TabsList>
              <TabsTrigger value="saved">
                {t('library.tabSaved')}
                {facets.data ? (
                  <span className="font-mono text-[10px] text-fg-subtle">{facets.data.total}</span>
                ) : null}
              </TabsTrigger>
              <TabsTrigger value="internet">
                <Globe className="h-3 w-3" aria-hidden />
                {t('library.tabInternet')}
              </TabsTrigger>
            </TabsList>
          </div>

          {tab === 'saved' ? (
            <FilterBar
              filter={filter}
              onFilterChange={setFilter}
              sort={sort}
              onSortChange={setSort}
              facets={facets.data}
            />
          ) : null}
        </div>

        <TabsContent value="saved" className="flex min-h-0 flex-1 flex-col pt-3">
          {sounds.isLoading ? (
            <EmptyState
              icon={Loader2}
              spinning
              title={t('library.loading')}
              description={t('common.loading')}
            />
          ) : (
            <VirtualGrid
              items={sounds.data ?? []}
              keyOf={(sound) => sound.id}
              empty={
                <EmptyState
                  icon={Upload}
                  title={searchText ? t('library.noResultsTitle') : t('library.emptyTitle')}
                  description={
                    searchText ? t('library.noResultsDescription') : t('library.emptyDescription')
                  }
                  action={
                    searchText ? null : (
                      <Button size="sm" onClick={() => void importWithDialog()}>
                        <Upload className="h-3.5 w-3.5" aria-hidden />
                        {t('library.importSounds')}
                      </Button>
                    )
                  }
                />
              }
              renderItem={(sound) => (
                <SoundRow
                  sound={sound}
                  isPreviewing={previewKey === `local:${sound.id}`}
                  isPlaying={playingSoundIds.includes(sound.id)}
                  isImageDropTarget={props.imageDropKey === `local:${sound.id}`}
                  onTogglePreview={(target) =>
                    previewKey === `local:${target.id}`
                      ? void playback.stopPreview()
                      : void playback.previewLocal(target.id)
                  }
                  onAssign={props.onAssignLocal}
                  onRename={props.onRenameSound}
                  onEditVolume={props.onEditSoundVolume}
                  onPickImage={(target) => void pickImageWithDialog(target.id)}
                  onClearImage={(target) => clearImage.mutate(target.id)}
                  onDelete={props.onDeleteSound}
                  onReveal={props.onRevealSound}
                  onOpenSource={(target) =>
                    target.sourcePageUrl ? props.onOpenUrl(target.sourcePageUrl) : undefined
                  }
                />
              )}
            />
          )}
        </TabsContent>

        <TabsContent value="internet" className="flex min-h-0 flex-1 flex-col pt-3">
          {!providersReady ? (
            <EmptyState
              icon={Settings2}
              title={t('library.noProvidersTitle')}
              description={t('library.noProvidersDescription')}
              action={
                <Button size="sm" onClick={() => props.onOpenSettings('providers')}>
                  {t('library.configureProviders')}
                </Button>
              }
            />
          ) : !remote.canSearch ? (
            <EmptyState
              icon={Globe}
              title={t('library.searchOnlineTitle')}
              description={t('library.searchOnlineDescription')}
            />
          ) : (
            <>
              {remoteErrors.length > 0 ? (
                <div className="mx-3 mb-2 flex flex-col gap-1 rounded-md border border-warning/40 bg-surface-1 px-2.5 py-2">
                  {remoteErrors.map((result) => (
                    <p
                      key={result.providerId}
                      className="flex items-start gap-1.5 text-[11px] text-warning"
                    >
                      <AlertTriangle className="mt-px h-3 w-3 shrink-0" aria-hidden />
                      <span>
                        <strong className="font-medium">{result.providerName}:</strong>{' '}
                        {result.error}
                      </span>
                    </p>
                  ))}
                </div>
              ) : null}

              {remote.isFetching && remoteItems.length === 0 ? (
                <EmptyState
                  icon={Loader2}
                  spinning
                  title={t('library.searching')}
                  description={t('library.searchingDescription')}
                />
              ) : (
                <VirtualGrid
                  items={remoteItems}
                  keyOf={(item) => `${item.providerId}:${item.remoteId}`}
                  empty={
                    <EmptyState
                      icon={Search}
                      title={t('library.noResultsTitle')}
                      description={t('library.noRemoteResults', { query: remote.query })}
                    />
                  }
                  renderItem={(item) => (
                    <RemoteRow
                      item={item}
                      status={remoteStatus(item)}
                      download={downloads[`${item.providerId}:${item.remoteId}`]}
                      savedSoundId={savedIds.get(`${item.providerId}:${item.remoteId}`)}
                      isImageDropTarget={
                        props.imageDropKey === `remote:${item.providerId}:${item.remoteId}`
                      }
                      onTogglePreview={(target) => {
                        const key = `remote:${target.providerId}:${target.remoteId}`;
                        // Cargando tambien se corta: es la unica forma de
                        // cancelar una descarga que esta tardando.
                        return previewKey === key || previewLoadingKey === key
                          ? void playback.stopPreview()
                          : void playback.previewRemote(target.providerId, target.remoteId);
                      }}
                      onDownload={(target) =>
                        download.mutate({
                          providerId: target.providerId,
                          remoteId: target.remoteId,
                        })
                      }
                      onAssign={props.onAssignRemote}
                      onOpenSource={(target) =>
                        target.sourcePageUrl ? props.onOpenUrl(target.sourcePageUrl) : undefined
                      }
                    />
                  )}
                />
              )}

              {remote.hasNextPage && remoteItems.length > 0 ? (
                <div className="border-t border-border-subtle p-2">
                  <Button
                    className="w-full justify-center"
                    variant="secondary"
                    onClick={() => void remote.fetchNextPage()}
                    disabled={remote.isFetchingNextPage}
                  >
                    {remote.isFetchingNextPage ? (
                      <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
                    ) : (
                      <ChevronDown className="h-4 w-4" aria-hidden />
                    )}
                    {t('library.loadMore')}
                  </Button>
                </div>
              ) : null}
            </>
          )}
        </TabsContent>
      </Tabs>
    </section>
  );
}
