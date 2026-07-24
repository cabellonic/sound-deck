import { ChevronLeft, ChevronRight, Square, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';

import { onAppEvent } from '@/lib/events';
import * as ipc from '@/lib/ipc';
import { cn, formatDuration } from '@/lib/utils';
import { useSlotKeys } from '@/features/useSlotKeys';
import type { AppSettings, PageSummary, SlotNumber, SoundPage } from '@/types/domain';

/**
 * Overlay compacto (§16).
 *
 * Se carga una sola vez al arrancar la aplicacion y queda oculto: mostrarlo es
 * instantaneo. Mientras tiene el foco captura las teclas 1 a 9, que por eso no
 * llegan al juego que estaba adelante.
 */
export function OverlayApp() {
  const [page, setPage] = useState<SoundPage | null>(null);
  const [pages, setPages] = useState<PageSummary[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [visible, setVisible] = useState(false);
  const [flashSlot, setFlashSlot] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (pageId?: string) => {
    try {
      const [nextPage, nextPages, nextSettings] = await Promise.all([
        ipc.getPage(pageId),
        ipc.listPages(),
        ipc.getSettings(),
      ]);
      setPage(nextPage);
      setPages(nextPages);
      setSettings(nextSettings);
      setError(null);
    } catch (caught) {
      setError(ipc.errorMessage(caught));
    }
  }, []);

  useEffect(() => {
    void refresh();

    const disposers = [
      onAppEvent('overlay-visibility-changed', (payload) => {
        setVisible(payload.visible);
        // Al abrirse, releemos por si cambio algo desde la ventana principal.
        if (payload.visible) void refresh();
      }),
      onAppEvent('page-changed', () => void refresh()),
      onAppEvent('slot-changed', () => void refresh(page?.id)),
      onAppEvent('settings-changed', (next) => setSettings(next)),
    ];

    return () => {
      for (const disposer of disposers) {
        void disposer.then((unlisten) => unlisten()).catch(() => undefined);
      }
    };
  }, [refresh, page?.id]);

  const close = useCallback(() => {
    void ipc.hideOverlay().catch(() => undefined);
  }, []);

  const changePage = useCallback(
    (delta: number) => {
      if (pages.length <= 1 || !page) return;
      const index = pages.findIndex((candidate) => candidate.id === page.id);
      const base = index >= 0 ? index : 0;
      const target = pages[(base + delta + pages.length) % pages.length];
      if (target)
        void ipc
          .setActivePage(target.id)
          .then(setPage)
          .catch(() => undefined);
    },
    [pages, page],
  );

  const play = useCallback(
    (slotNumber: SlotNumber) => {
      if (!page) return;

      const slot = page.slots.find((candidate) => candidate.slotNumber === slotNumber);
      if (!slot?.sound) return;

      setFlashSlot(slotNumber);
      window.setTimeout(() => setFlashSlot(null), 180);

      void ipc
        .playSlot(page.id, slotNumber)
        .then(() => {
          if (settings?.general.closeOverlayAfterPlay !== false) close();
        })
        .catch((caught: unknown) => setError(ipc.errorMessage(caught)));
    },
    [page, settings, close],
  );

  useSlotKeys({
    enabled: true,
    allowRepeat: settings?.shortcuts.allowKeyRepeat ?? false,
    onSlot: play,
    onPrevPage: () => changePage(-1),
    onNextPage: () => changePage(1),
    onEscape: close,
  });

  const index = page ? pages.findIndex((candidate) => candidate.id === page.id) : -1;

  return (
    <div className="overlay-root flex h-full items-center justify-center p-3">
      <div
        className={cn(
          'flex w-full flex-col gap-2.5 rounded-panel border border-border-strong',
          'bg-surface-1/95 p-3 shadow-2xl backdrop-blur',
        )}
        role="dialog"
        aria-label="Overlay de Sound Deck"
        aria-live="polite"
      >
        <header className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => changePage(-1)}
            disabled={pages.length <= 1}
            aria-label="Pagina anterior"
            className="rounded p-1 text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg-default disabled:opacity-40"
          >
            <ChevronLeft className="h-4 w-4" aria-hidden />
          </button>

          <div className="min-w-0 flex-1 text-center">
            <p className="truncate text-sm font-medium text-fg-default">
              {page?.name ?? 'Sin pagina'}
            </p>
            <p className="font-mono text-[10px] tabular-nums text-fg-subtle">
              {index >= 0 ? index + 1 : 0} / {pages.length}
            </p>
          </div>

          <button
            type="button"
            onClick={() => changePage(1)}
            disabled={pages.length <= 1}
            aria-label="Pagina siguiente"
            className="rounded p-1 text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg-default disabled:opacity-40"
          >
            <ChevronRight className="h-4 w-4" aria-hidden />
          </button>

          <button
            type="button"
            onClick={close}
            aria-label="Cerrar overlay"
            className="rounded p-1 text-fg-subtle transition-colors hover:bg-surface-3 hover:text-fg-default"
          >
            <X className="h-4 w-4" aria-hidden />
          </button>
        </header>

        {error ? (
          <p className="rounded border border-danger/50 bg-danger-soft px-2 py-1 text-[11px] text-danger">
            {error}
          </p>
        ) : null}

        <div className="grid grid-cols-3 gap-1.5" role="group" aria-label="Botones de la pagina">
          {(page?.slots ?? []).map((slot) => {
            const label = slot.customLabel ?? slot.sound?.name ?? null;
            const duration = formatDuration(slot.sound?.durationMs);
            const broken = Boolean(slot.sound && !slot.sound.fileAvailable);
            // La imagen del audio tambien aca: el overlay es lo que se ve
            // encima del juego, donde reconocer el boton de un vistazo importa
            // mas que en ningun otro lado.
            const imageSrc = ipc.soundImageSrc(slot.sound);

            return (
              <button
                key={slot.slotNumber}
                type="button"
                onClick={() => play(slot.slotNumber)}
                disabled={!label}
                aria-label={`Boton ${slot.slotNumber}. ${label ?? 'Sin asignar'}`}
                className={cn(
                  'relative flex aspect-[4/3] flex-col justify-between overflow-hidden rounded-md border p-1.5 text-left transition-colors',
                  label
                    ? 'border-border-strong bg-surface-2 hover:bg-surface-3'
                    : 'border-dashed border-border-subtle bg-surface-0 opacity-60',
                  broken && 'border-danger/60',
                  flashSlot === slot.slotNumber && 'border-accent bg-accent-soft',
                )}
              >
                {imageSrc ? (
                  <>
                    <img
                      src={imageSrc}
                      alt=""
                      aria-hidden
                      className="pointer-events-none absolute inset-0 h-full w-full object-cover"
                    />
                    <span
                      aria-hidden
                      className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/85 via-black/35 to-black/10"
                    />
                  </>
                ) : null}

                <span
                  className={cn(
                    'relative font-mono text-[10px] font-semibold',
                    imageSrc ? 'text-white/90 drop-shadow' : 'text-fg-subtle',
                  )}
                >
                  {slot.slotNumber}
                </span>
                <span className="relative min-w-0">
                  <span
                    className={cn(
                      'block truncate text-[11px] font-medium leading-tight',
                      imageSrc
                        ? 'text-white drop-shadow-[0_1px_2px_rgba(0,0,0,0.9)]'
                        : 'text-fg-default',
                    )}
                  >
                    {label ?? '—'}
                  </span>
                  {duration ? (
                    <span
                      className={cn(
                        'block font-mono text-[9px] tabular-nums',
                        imageSrc ? 'text-white/80 drop-shadow' : 'text-fg-subtle',
                      )}
                    >
                      {duration}
                    </span>
                  ) : null}
                </span>
              </button>
            );
          })}
        </div>

        <footer className="flex items-center justify-between gap-2 text-[10px] text-fg-subtle">
          <span>1-9 reproduce · Esc cierra</span>
          <button
            type="button"
            onClick={() => void ipc.stopAll().catch(() => undefined)}
            className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 transition-colors hover:bg-surface-3 hover:text-fg-default"
          >
            <Square className="h-3 w-3" aria-hidden />
            Detener todo
          </button>
        </footer>

        {/* Estado invisible que confirma si el overlay se considera visible. */}
        <span className="sr-only" data-testid="overlay-visible">
          {visible ? 'visible' : 'oculto'}
        </span>
      </div>
    </div>
  );
}
