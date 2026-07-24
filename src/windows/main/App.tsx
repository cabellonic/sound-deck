import { useQuery } from '@tanstack/react-query';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { Layers, Loader2, Settings, Square } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';

import { DragLayer } from '@/components/dnd/DragLayer';
import { AssignToSlotDialog } from '@/components/library/AssignToSlotDialog';
import { LibraryPanel } from '@/components/library/LibraryPanel';
import { PageBar } from '@/components/soundboard/PageBar';
import { SlotGrid } from '@/components/soundboard/SlotGrid';
import { Button } from '@/components/ui/Button';
import {
  ConfirmDialog,
  PromptDialog,
  SoundDetailsDialog,
  VolumeDialog,
} from '@/components/ui/dialogs';
import { Tooltip, TooltipProvider } from '@/components/ui/primitives';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { Toaster } from '@/components/ui/Toaster';
import { queryKeys } from '@/features/queryKeys';
import { useAppEvents } from '@/features/useAppEvents';
import { useFileDrop } from '@/features/useFileDrop';
import { useLibraryMutations } from '@/features/useLibrary';
import { useNativeContextMenu } from '@/features/useNativeContextMenu';
import { useSlotKeys } from '@/features/useSlotKeys';
import {
  usePage,
  usePageMutations,
  usePages,
  usePlaybackActions,
  useSlotMutations,
} from '@/features/useSoundboard';
import { useTheme } from '@/features/useTheme';
import type { DragPayload } from '@/lib/drag';
import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { volumeToPercent } from '@/lib/utils';
import { useUiStore } from '@/stores/useUiStore';
import type {
  PageSummary,
  RemoteSound,
  SlotNumber,
  Sound,
  SoundSlot,
  SoundUsage,
} from '@/types/domain';

import { Onboarding } from './Onboarding';

/** Estado de los dialogos, como una union discriminada en vez de booleanos sueltos. */
type DialogState =
  | { kind: 'none' }
  | { kind: 'create-page' }
  | { kind: 'rename-page'; page: PageSummary }
  | { kind: 'delete-page'; page: PageSummary; assigned: number }
  | { kind: 'rename-sound'; sound: Sound }
  | { kind: 'sound-volume'; sound: Sound }
  | { kind: 'delete-sound'; sound: Sound; usage: SoundUsage[] }
  | { kind: 'slot-label'; slot: SoundSlot }
  | { kind: 'slot-volume'; slot: SoundSlot }
  | { kind: 'slot-details'; slot: SoundSlot }
  | { kind: 'assign-local'; sound: Sound }
  | { kind: 'assign-remote'; item: RemoteSound };

export function App() {
  const [dialog, setDialog] = useState<DialogState>({ kind: 'none' });
  const [slotDownloads, setSlotDownloads] = useState<Record<number, number>>({});

  const settingsOpen = useUiStore((state) => state.settingsOpen);
  const setSettingsOpen = useUiStore((state) => state.setSettingsOpen);
  const openSettings = useUiStore((state) => state.openSettings);
  const playingSoundIds = useUiStore((state) => state.playingSoundIds);
  const previewKey = useUiStore((state) => state.previewKey);
  const pushToast = useUiStore((state) => state.pushToast);

  useAppEvents();
  useNativeContextMenu();

  const appState = useQuery({ queryKey: queryKeys.appState, queryFn: ipc.getAppState });
  const settings = useQuery({
    queryKey: queryKeys.settings,
    queryFn: ipc.getSettings,
    initialData: appState.data?.settings,
  });

  useTheme(settings.data?.general.theme);

  // Lo que suena un audio linkeado. Los dialogos de volumen lo muestran para
  // que se vea a que nivel equivale seguir el general.
  const masterVolume = settings.data?.audio.masterVolume ?? 0.35;

  const pages = usePages();
  const [activePageId, setActivePageId] = useState<string | null>(null);
  const currentPageId = activePageId ?? appState.data?.activePage?.id ?? null;
  const page = usePage(currentPageId ?? undefined);

  const pageMutations = usePageMutations();
  const slotMutations = useSlotMutations();
  const library = useLibraryMutations();
  const playback = usePlaybackActions();

  const pageList = pages.data ?? appState.data?.pages ?? [];
  const currentPage = page.data ?? appState.data?.activePage ?? undefined;

  const selectPage = useCallback(
    (pageId: string) => {
      setActivePageId(pageId);
      pageMutations.activate.mutate(pageId);
    },
    [pageMutations.activate],
  );

  const goRelativePage = useCallback(
    (delta: number) => {
      if (pageList.length <= 1) return;
      const index = pageList.findIndex((candidate) => candidate.id === currentPage?.id);
      const base = index >= 0 ? index : 0;
      const target = pageList[(base + delta + pageList.length) % pageList.length];
      if (target) selectPage(target.id);
    },
    [pageList, currentPage?.id, selectPage],
  );

  const playSlot = useCallback(
    (slotNumber: SlotNumber) => {
      if (currentPage) void playback.playSlot(currentPage.id, slotNumber);
    },
    [currentPage, playback],
  );

  // Las teclas 1-9 reproducen mientras el foco no este en un campo de texto.
  useSlotKeys({
    enabled: Boolean(currentPage) && !settingsOpen && dialog.kind === 'none',
    allowRepeat: settings.data?.shortcuts.allowKeyRepeat ?? false,
    onSlot: playSlot,
    onPrevPage: () => goRelativePage(-1),
    onNextPage: () => goRelativePage(1),
  });

  const openUrlSafely = (url: string) => {
    void openUrl(url).catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  /**
   * Abre el explorador con el archivo seleccionado. `revealItemInDir` resuelve
   * la carpeta contenedora en cada sistema operativo; no partimos rutas a mano.
   */
  const revealSound = (sound: Sound) => {
    void ipc
      .revealSoundInFolder(sound.id)
      .then((path) => revealItemInDir(path))
      .catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  /** Un drop sobre un slot puede venir de la biblioteca, de Internet o de otro slot. */
  const handleDropPayload = (slotNumber: SlotNumber, payload: DragPayload) => {
    if (!currentPage) return;

    switch (payload.kind) {
      case 'local-sound':
        slotMutations.assign.mutate({
          pageId: currentPage.id,
          slotNumber,
          soundId: payload.soundId,
        });
        break;

      case 'slot':
        slotMutations.swap.mutate({
          fromPageId: payload.pageId,
          fromSlot: payload.slotNumber,
          toPageId: currentPage.id,
          toSlot: slotNumber,
        });
        break;

      case 'remote-sound': {
        // Descarga + asignacion en una sola operacion transaccional (§7).
        setSlotDownloads((current) => ({ ...current, [slotNumber]: 0 }));
        slotMutations.downloadAndAssign
          .mutateAsync({
            providerId: payload.providerId,
            remoteId: payload.remoteId,
            pageId: currentPage.id,
            slotNumber,
          })
          .finally(() =>
            setSlotDownloads((current) => {
              const next = { ...current };
              delete next[slotNumber];
              return next;
            }),
          )
          .catch(() => {
            // El error ya se informa desde la mutacion.
          });
        break;
      }
    }
  };

  /** Archivos del sistema soltados sobre un boton: se importan y se asigna el primero. */
  const handleDropFilesOnSlot = useCallback(
    (slotNumber: SlotNumber, paths: string[]) => {
      const pageId = currentPage?.id;
      if (!pageId) return;

      void library.importFiles
        .mutateAsync(paths)
        .then((report) => {
          const first = report.imported[0] ?? report.duplicates[0];
          if (first) {
            slotMutations.assign.mutate({ pageId, slotNumber, soundId: first.id });
          }
        })
        .catch(() => {
          // El error ya se informa desde la mutacion.
        });
    },
    [currentPage?.id, library.importFiles, slotMutations.assign],
  );

  const handleDropFilesOnLibrary = useCallback(
    (paths: string[]) => library.importFiles.mutate(paths),
    [library.importFiles],
  );

  const fileDrop = useFileDrop({
    onDropOnSlot: handleDropFilesOnSlot,
    onDropOnLibrary: handleDropFilesOnLibrary,
  });

  const askDeleteSound = (sound: Sound) => {
    void ipc
      .getSoundUsage(sound.id)
      .then((usage) => setDialog({ kind: 'delete-sound', sound, usage }))
      .catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  const askDeletePage = (target: PageSummary) => {
    void ipc
      .countPageAssignments(target.id)
      .then((assigned) => setDialog({ kind: 'delete-page', page: target, assigned }))
      .catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  const assignTarget = useMemo(() => {
    if (dialog.kind === 'assign-local') return dialog.sound.name;
    if (dialog.kind === 'assign-remote') return dialog.item.title;
    return '';
  }, [dialog]);

  if (appState.isLoading) {
    return (
      <div className="flex h-full items-center justify-center gap-2 text-sm text-fg-muted">
        <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
        Iniciando Sound Deck...
      </div>
    );
  }

  if (appState.isError) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
        <p className="text-sm text-danger">{errorMessage(appState.error)}</p>
        <Button onClick={() => void appState.refetch()}>Reintentar</Button>
      </div>
    );
  }

  const version = appState.data?.version ?? '';
  const showOnboarding = settings.data?.general.onboardingCompleted === false;
  const isAnythingPlaying = playingSoundIds.length > 0 || previewKey !== null;

  return (
    <TooltipProvider>
      <div className="flex h-full flex-col">
        <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border-subtle bg-surface-1 px-3">
          <h1 className="text-sm font-semibold tracking-tight">Sound Deck</h1>
          <span className="font-mono text-[10px] text-fg-subtle">{version}</span>

          <div className="ml-auto flex items-center gap-1.5">
            <Tooltip
              content={
                isAnythingPlaying
                  ? 'Detener todos los sonidos'
                  : 'No hay nada sonando en este momento'
              }
            >
              {/* El tooltip necesita un elemento que reciba eventos: un boton
                  deshabilitado no los emite, por eso va envuelto. */}
              <span className="inline-flex">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={!isAnythingPlaying}
                  onClick={() => void playback.stopAll()}
                >
                  <Square className="h-3.5 w-3.5" aria-hidden />
                  Detener
                </Button>
              </span>
            </Tooltip>

            <Tooltip
              content={`Abrir overlay (${settings.data?.shortcuts.bindings.find((b) => b.action === 'toggle_overlay')?.accelerator ?? 'Ctrl+Alt+Space'})`}
            >
              <Button
                variant="secondary"
                size="sm"
                onClick={() =>
                  void ipc
                    .showOverlay()
                    .catch((error: unknown) => pushToast('error', errorMessage(error)))
                }
              >
                <Layers className="h-3.5 w-3.5" aria-hidden />
                Overlay
              </Button>
            </Tooltip>

            <Tooltip content="Configuracion">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setSettingsOpen(true)}
                aria-label="Abrir configuracion"
              >
                <Settings className="h-4 w-4" aria-hidden />
              </Button>
            </Tooltip>
          </div>
        </header>

        <main className="flex min-h-0 flex-1">
          <section
            className="flex w-[22rem] shrink-0 flex-col gap-3 overflow-y-auto p-3"
            aria-label="Botonera"
          >
            <PageBar
              pages={pageList}
              activePageId={currentPage?.id ?? null}
              onSelect={selectPage}
              onCreate={() => setDialog({ kind: 'create-page' })}
              onRename={(target) => setDialog({ kind: 'rename-page', page: target })}
              onDelete={askDeletePage}
              onDuplicate={(target) => pageMutations.duplicate.mutate(target.id)}
              onReorder={(ids) => pageMutations.reorder.mutate(ids)}
            />

            {currentPage ? (
              <SlotGrid
                page={currentPage}
                playingSoundIds={playingSoundIds}
                slotDownloads={slotDownloads}
                onPlay={playSlot}
                onDropPayload={handleDropPayload}
                onClear={(slotNumber) =>
                  slotMutations.clear.mutate({ pageId: currentPage.id, slotNumber })
                }
                onEditLabel={(slot) => setDialog({ kind: 'slot-label', slot })}
                onEditVolume={(slot) => setDialog({ kind: 'slot-volume', slot })}
                onPickImage={(sound) => void library.pickImageWithDialog(sound.id)}
                onClearImage={(sound) => library.clearImage.mutate(sound.id)}
                onReveal={(soundId) => {
                  const target = currentPage.slots.find((slot) => slot.sound?.id === soundId);
                  if (target?.sound) revealSound(target.sound);
                }}
                onShowDetails={(slot) => setDialog({ kind: 'slot-details', slot })}
              />
            ) : (
              <p className="text-sm text-fg-subtle">No hay ninguna pagina activa.</p>
            )}
          </section>

          <LibraryPanel
            onAssignLocal={(sound) => setDialog({ kind: 'assign-local', sound })}
            onAssignRemote={(item) => setDialog({ kind: 'assign-remote', item })}
            onRenameSound={(sound) => setDialog({ kind: 'rename-sound', sound })}
            onEditSoundVolume={(sound) => setDialog({ kind: 'sound-volume', sound })}
            onDeleteSound={askDeleteSound}
            onRevealSound={revealSound}
            onOpenUrl={openUrlSafely}
            onOpenSettings={openSettings}
            fileDropActive={fileDrop.isOver && fileDrop.hoveredSlot === null}
          />
        </main>
      </div>

      {settings.data ? (
        <SettingsDialog
          open={settingsOpen}
          onOpenChange={setSettingsOpen}
          settings={settings.data}
          version={version}
        />
      ) : null}

      {showOnboarding ? (
        <Onboarding
          onImport={() => void library.importWithDialog()}
          onOpenSettings={() => openSettings('providers')}
        />
      ) : null}

      <PromptDialog
        open={dialog.kind === 'create-page'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Nueva pagina"
        description="Las paginas son colecciones libres: pueden mezclar audios de cualquier origen."
        label="Nombre"
        initialValue=""
        confirmLabel="Crear"
        onConfirm={(name) => pageMutations.create.mutate(name)}
      />

      <PromptDialog
        open={dialog.kind === 'rename-page'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Renombrar pagina"
        label="Nombre"
        initialValue={dialog.kind === 'rename-page' ? dialog.page.name : ''}
        onConfirm={(name) => {
          if (dialog.kind === 'rename-page') {
            pageMutations.rename.mutate({ pageId: dialog.page.id, name });
          }
        }}
      />

      <ConfirmDialog
        open={dialog.kind === 'delete-page'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Borrar pagina"
        description={
          dialog.kind === 'delete-page'
            ? `Se va a borrar "${dialog.page.name}". Los audios siguen en la biblioteca.`
            : ''
        }
        details={
          dialog.kind === 'delete-page' && dialog.assigned > 0 ? (
            <p>
              Esta pagina tiene <strong>{dialog.assigned}</strong>{' '}
              {dialog.assigned === 1 ? 'boton asignado' : 'botones asignados'}.
            </p>
          ) : undefined
        }
        confirmLabel="Borrar pagina"
        onConfirm={() => {
          if (dialog.kind === 'delete-page') pageMutations.remove.mutate(dialog.page.id);
        }}
      />

      <PromptDialog
        open={dialog.kind === 'rename-sound'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Renombrar audio"
        label="Nombre"
        initialValue={dialog.kind === 'rename-sound' ? dialog.sound.name : ''}
        onConfirm={(name) => {
          if (dialog.kind === 'rename-sound') {
            library.rename.mutate({ soundId: dialog.sound.id, name });
          }
        }}
      />

      <VolumeDialog
        open={dialog.kind === 'sound-volume'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Volumen del audio"
        description="Vale para este audio en toda la aplicacion."
        value={dialog.kind === 'sound-volume' ? dialog.sound.customVolume : null}
        inheritedVolume={masterVolume}
        inheritLabel="Seguir el volumen general"
        inheritHint={`Al desactivarlo, este audio suena siempre al valor que elijas, aunque muevas el general (ahora en ${volumeToPercent(masterVolume)}%).`}
        onConfirm={(volume) => {
          if (dialog.kind === 'sound-volume') {
            library.setVolume.mutate({ soundId: dialog.sound.id, volume });
          }
        }}
      />

      <ConfirmDialog
        open={dialog.kind === 'delete-sound'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Eliminar audio"
        description={
          dialog.kind === 'delete-sound'
            ? `Se va a borrar "${dialog.sound.name}" de la biblioteca y del disco.`
            : ''
        }
        details={
          dialog.kind === 'delete-sound' && dialog.usage.length > 0 ? (
            <div>
              <p className="mb-1">Se va a quitar de estos botones:</p>
              <ul className="list-inside list-disc text-xs">
                {dialog.usage.map((usage) => (
                  <li key={`${usage.pageId}-${usage.slotNumber}`}>
                    {usage.pageName} — boton {usage.slotNumber}
                  </li>
                ))}
              </ul>
            </div>
          ) : undefined
        }
        confirmLabel="Eliminar"
        onConfirm={() => {
          if (dialog.kind === 'delete-sound') library.remove.mutate(dialog.sound.id);
        }}
      />

      <PromptDialog
        open={dialog.kind === 'slot-label'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Nombre visible del boton"
        description="Solo cambia lo que se muestra en este boton, no el nombre del audio."
        label="Etiqueta"
        initialValue={
          dialog.kind === 'slot-label'
            ? (dialog.slot.customLabel ?? dialog.slot.sound?.name ?? '')
            : ''
        }
        onConfirm={(label) => {
          if (dialog.kind === 'slot-label') {
            slotMutations.setLabel.mutate({
              pageId: dialog.slot.pageId,
              slotNumber: dialog.slot.slotNumber,
              label,
            });
          }
        }}
      />

      <VolumeDialog
        open={dialog.kind === 'slot-volume'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        title="Volumen del boton"
        description="Solo cambia como suena en este boton."
        value={dialog.kind === 'slot-volume' ? dialog.slot.customVolume : null}
        inheritedVolume={
          dialog.kind === 'slot-volume'
            ? (dialog.slot.sound?.customVolume ?? masterVolume)
            : masterVolume
        }
        inheritLabel="Usar el volumen del audio"
        inheritHint="Al desactivarlo, este boton queda fijo en el valor que elijas, aunque el audio suene distinto en otros botones."
        onConfirm={(volume) => {
          if (dialog.kind === 'slot-volume') {
            slotMutations.setVolume.mutate({
              pageId: dialog.slot.pageId,
              slotNumber: dialog.slot.slotNumber,
              volume,
            });
          }
        }}
      />

      <SoundDetailsDialog
        open={dialog.kind === 'slot-details'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        sound={dialog.kind === 'slot-details' ? dialog.slot.sound : null}
      />

      <AssignToSlotDialog
        open={dialog.kind === 'assign-local' || dialog.kind === 'assign-remote'}
        onOpenChange={(open) => !open && setDialog({ kind: 'none' })}
        soundName={assignTarget}
        pages={pageList}
        currentPage={currentPage}
        defaultPageId={currentPage?.id ?? null}
        onSelectPage={selectPage}
        onConfirm={(pageId, slotNumber) => {
          if (dialog.kind === 'assign-local') {
            slotMutations.assign.mutate({ pageId, slotNumber, soundId: dialog.sound.id });
          } else if (dialog.kind === 'assign-remote') {
            setSlotDownloads((current) => ({ ...current, [slotNumber]: 0 }));
            slotMutations.downloadAndAssign
              .mutateAsync({
                providerId: dialog.item.providerId,
                remoteId: dialog.item.remoteId,
                pageId,
                slotNumber,
              })
              .finally(() =>
                setSlotDownloads((current) => {
                  const next = { ...current };
                  delete next[slotNumber];
                  return next;
                }),
              )
              .catch(() => undefined);
          }
          setDialog({ kind: 'none' });
        }}
      />

      <DragLayer />
      <Toaster />
    </TooltipProvider>
  );
}
