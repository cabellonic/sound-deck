import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ChevronLeft, ChevronRight, Grip, MoveDiagonal, Square, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { onAppEvent } from '@/lib/events';
import * as ipc from '@/lib/ipc';
import { cn, formatDuration } from '@/lib/utils';
import { useSlotKeys } from '@/features/useSlotKeys';
import { resolveLocale } from '@/i18n';
import { useTranslator } from '@/i18n/useTranslation';
import type { AppSettings, PageSummary, SlotNumber, SoundPage } from '@/types/domain';

/**
 * Ancho para el que esta pensado el overlay, en pixeles logicos.
 *
 * Espejo del `width` de la ventana en `tauri.conf.json`: a ese ancho la escala
 * es 1 y todo mide lo que dicen las clases.
 */
const BASE_WIDTH = 520;

/**
 * Hasta donde se puede estirar, en pixeles logicos. Espejo de `MIN_SIZE` y
 * `MAX_SIZE` en `overlay/mod.rs`, que son el limite de verdad.
 *
 * El minimo es generoso a proposito: achicarlo hasta que solo se distingan las
 * imagenes es una decision valida de quien lo usa.
 */
const MIN_WIDTH = 300;
const MAX_WIDTH = 1100;

/**
 * Barra que aparece solo mientras se ajusta el overlay.
 *
 * Mover y redimensionar los hace el sistema (`startDragging` y
 * `startResizeDragging`): el overlay no tiene barra de titulo ni bordes, asi
 * que sin esto no habria de donde agarrarlo.
 */
function PlacementBar({
  t,
  size,
  faded,
  onSave,
}: {
  t: ReturnType<typeof useTranslator>['t'];
  size: { width: number; height: number };
  /** Mientras se estira la esquina la barra se aparta para no tapar nada. */
  faded: boolean;
  onSave: () => void;
}) {
  return (
    <div
      className={cn(
        // Flotante y no en el flujo: si empujara el contenido, el overlay que
        // se ve mientras se lo ajusta no seria el que despues se abre.
        'absolute inset-x-3 top-3 z-10 flex flex-col gap-2 rounded-md p-2',
        'border-2 border-accent bg-accent-soft shadow-xl transition-opacity duration-150',
        faded && 'pointer-events-none opacity-10',
      )}
    >
      <div className="flex items-center gap-2 text-[0.6875rem] text-fg-default">
        <Grip className="h-4 w-4 shrink-0 text-accent-strong" aria-hidden />
        <span className="min-w-0 flex-1">{t('overlay.placementHint')}</span>
        <span className="shrink-0 rounded bg-surface-1/70 px-1.5 py-0.5 font-mono tabular-nums text-fg-muted">
          {size.width} × {size.height}
        </span>
      </div>

      <div className="flex items-center justify-between gap-1.5" data-placement-control>
        <span className="text-[0.625rem] text-fg-muted">{t('overlay.placementResizeHint')}</span>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => void ipc.cancelOverlayPlacement().catch(() => undefined)}
            className="rounded px-2 py-1 text-[0.6875rem] text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg-default"
          >
            {t('common.cancel')}
          </button>
          <button
            type="button"
            onClick={onSave}
            className="rounded bg-accent px-2 py-1 text-[0.6875rem] font-medium text-surface-0 transition-colors hover:bg-accent-strong"
          >
            {t('overlay.placementSave')}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Agarradera de la esquina inferior derecha, la unica forma de redimensionar:
 * la ventana no tiene bordes visibles donde apuntar.
 *
 * Va pegada a la esquina de la ventana, no a la del panel: lo que se estira es
 * la ventana, y el panel se acomoda adentro igual que cuando el overlay se
 * abre de verdad.
 *
 * El arrastre lo lleva el overlay y no el sistema (`startResizeDragging`)
 * porque solo asi se puede imponer la proporcion: mientras dura el bucle de
 * redimensionado de Windows, cualquier alto que pongamos lo pisa el siguiente
 * movimiento del mouse.
 */
function ResizeGrip({
  label,
  onStart,
  onMove,
  onEnd,
}: {
  label: string;
  onStart: (event: React.PointerEvent<HTMLElement>) => void;
  onMove: (event: React.PointerEvent<HTMLElement>) => void;
  onEnd: (event: React.PointerEvent<HTMLElement>) => void;
}) {
  return (
    <button
      type="button"
      data-placement-control
      aria-label={label}
      title={label}
      onPointerDown={onStart}
      onPointerMove={onMove}
      onPointerUp={onEnd}
      onPointerCancel={onEnd}
      className={cn(
        'absolute bottom-0 right-0 z-10 flex h-6 w-6 cursor-nwse-resize items-center justify-center',
        'rounded-tl-md rounded-br-panel bg-accent text-surface-0 shadow-lg',
      )}
    >
      <MoveDiagonal className="h-3.5 w-3.5 rotate-90" aria-hidden />
    </button>
  );
}

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
  const [placing, setPlacing] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [resizing, setResizing] = useState(false);
  const [size, setSize] = useState({ width: BASE_WIDTH, height: 0 });
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  /** Alto sobre ancho del ultimo dibujo, para estimar el alto al estirar. */
  const aspect = useRef(460 / BASE_WIDTH);
  const resizeStart = useRef<{ pointerId: number; screenX: number; width: number } | null>(null);
  const pendingWidth = useRef<number | null>(null);
  const frame = useRef(0);

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

  /** Alto en pixeles logicos que ocupa el overlay dibujado, con su margen. */
  const measureFitHeight = useCallback(() => {
    const root = rootRef.current;
    const panel = panelRef.current;
    if (!root || !panel) return null;

    const style = window.getComputedStyle(root);
    const padding = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
    return Math.ceil(panel.getBoundingClientRect().height + padding);
  }, []);

  // El tamano del overlay lo elige el usuario, asi que todo se mide contra el
  // ancho de la ventana: cambiando el tamano de fuente de la raiz, las clases
  // en `rem` (espacios, iconos y texto) crecen juntas en vez de quedar chicas
  // dentro de un overlay grande.
  const applyScale = useCallback(() => {
    // El piso acompana al ancho minimo: si la escala se frenara antes, el
    // texto dejaria de encoger y la proporcion se torceria justo ahi.
    const scale = Math.min(2, Math.max(0.55, window.innerWidth / BASE_WIDTH));
    document.documentElement.style.fontSize = `${16 * scale}px`;
    setSize({
      width: Math.round(window.innerWidth),
      // El alto sale del contenido y no de la ventana: son el mismo numero
      // salvo en el instante entre que se estira y `fitWindowHeight` la iguala.
      height: measureFitHeight() ?? Math.round(window.innerHeight),
    });
  }, [measureFitHeight]);

  /**
   * Deja la ventana con el alto exacto que ocupa el overlay dibujado.
   *
   * Es lo que fuerza la proporcion: el ancho lo elige quien lo usa y el alto
   * sale del contenido, asi que no hay tamano posible que corte un boton.
   */
  const fitWindowHeight = useCallback(() => {
    const fit = measureFitHeight();
    if (fit === null || window.innerWidth === 0) return;

    aspect.current = fit / window.innerWidth;
    // La tolerancia evita que un redondeo de medio pixel dispare otro cambio
    // de tamano, que volveria a medir, y asi para siempre.
    if (Math.abs(fit - window.innerHeight) <= 2) return;

    void getCurrentWindow()
      .setSize(new LogicalSize(Math.round(window.innerWidth), fit))
      .catch(() => undefined);
  }, [measureFitHeight]);

  useEffect(() => {
    const onResize = () => {
      applyScale();
      fitWindowHeight();
    };

    onResize();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [applyScale, fitWindowHeight]);

  // Un `setSize` por cuadro como mucho: el puntero manda muchos mas eventos de
  // los que la ventana puede seguir, y encolarlos la deja atras del cursor.
  const requestWidth = useCallback((width: number) => {
    pendingWidth.current = width;
    if (frame.current) return;

    frame.current = window.requestAnimationFrame(() => {
      frame.current = 0;
      const next = pendingWidth.current;
      if (next === null) return;

      void getCurrentWindow()
        .setSize(new LogicalSize(next, Math.round(next * aspect.current)))
        .catch(() => undefined);
    });
  }, []);

  const beginResize = useCallback((event: React.PointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    resizeStart.current = {
      pointerId: event.pointerId,
      screenX: event.screenX,
      width: window.innerWidth,
    };
    setResizing(true);
  }, []);

  const moveResize = useCallback(
    (event: React.PointerEvent<HTMLElement>) => {
      const start = resizeStart.current;
      if (!start || start.pointerId !== event.pointerId) return;

      // Solo el movimiento horizontal cuenta: el vertical no tiene nada que
      // decidir, el alto ya viene dado por el ancho.
      const width = start.width + (event.screenX - start.screenX);
      requestWidth(Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width))));
    },
    [requestWidth],
  );

  const endResize = useCallback((event: React.PointerEvent<HTMLElement>) => {
    resizeStart.current = null;
    setResizing(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }, []);

  useEffect(() => {
    void refresh();

    const disposers = [
      onAppEvent('overlay-visibility-changed', (payload) => {
        setVisible(payload.visible);
        if (payload.visible) {
          // Al abrirse, releemos por si cambio algo desde la ventana principal.
          void refresh();
          // La ventana pudo cambiar de tamano mientras estaba oculta.
          applyScale();
        }
      }),
      onAppEvent('page-changed', () => void refresh()),
      onAppEvent('slot-changed', () => void refresh(page?.id)),
      onAppEvent('settings-changed', (next) => setSettings(next)),
      onAppEvent('overlay-placement-changed', (payload) => {
        setPlacing(payload.placing);
        // Cancelar con el puntero apretado desmonta la agarradera sin que
        // llegue el `pointerup`: sin esto, el proximo ajuste abriria creyendo
        // que se lo esta estirando.
        if (!payload.placing) {
          resizeStart.current = null;
          setResizing(false);
        }
      }),
    ];

    return () => {
      for (const disposer of disposers) {
        void disposer.then((unlisten) => unlisten()).catch(() => undefined);
      }
    };
  }, [refresh, applyScale, page?.id]);

  const close = useCallback(() => {
    void ipc.hideOverlay().catch(() => undefined);
  }, []);

  // Al guardar, la ventana se recorta al alto que ocupa el panel: el ancho lo
  // eligio el usuario, pero el alto sobrante seria un pedazo transparente que
  // igual se come los clics encima del juego.
  const savePlacement = useCallback(() => {
    void ipc.saveOverlayPlacement(measureFitHeight() ?? undefined).catch(() => undefined);
  }, [measureFitHeight]);

  // Mientras se ajusta, todo el panel es la agarradera: no hay que adivinar
  // desde donde se arrastra. Los controles de la barra se excluyen.
  const startDragging = useCallback((event: React.PointerEvent<HTMLElement>) => {
    const target = event.target;
    if (event.button !== 0 || !(target instanceof Element)) return;
    if (target.closest('[data-placement-control]')) return;

    setDragging(true);
    void getCurrentWindow()
      .startDragging()
      .catch(() => undefined)
      .finally(() => setDragging(false));
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
      // Colocando el overlay, un clic o una tecla solo lo estan arrastrando.
      if (!page || placing) return;

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
    [page, settings, close, placing],
  );

  // El hook sigue activo mientras se coloca para que Escape cancele; lo que no
  // suena es `play`, que se guarda solo.
  useSlotKeys({
    enabled: true,
    allowRepeat: settings?.shortcuts.allowKeyRepeat ?? false,
    onSlot: play,
    onPrevPage: () => changePage(-1),
    onNextPage: () => changePage(1),
    onEscape: placing ? () => void ipc.cancelOverlayPlacement().catch(() => undefined) : close,
  });

  // El overlay lee la configuracion por su cuenta: no monta el cliente de
  // queries, asi que resuelve el idioma a mano en vez de usar `useTranslation`.
  const { t } = useTranslator(resolveLocale(settings?.general.language));

  const index = page ? pages.findIndex((candidate) => candidate.id === page.id) : -1;

  return (
    <div
      ref={rootRef}
      className={cn(
        'overlay-root relative flex h-full items-center justify-center p-3',
        // El punteado marca donde termina la ventana: es lo que se estira desde
        // la esquina, y el panel de adentro queda igual que al abrirlo. Va como
        // `outline` y no como borde porque el borde correria el panel 4px y el
        // overlay dejaria de ser identico al que se abre despues.
        placing && 'rounded-panel outline-2 -outline-offset-2 outline-dashed outline-accent/60',
        placing && (dragging ? 'cursor-grabbing' : 'cursor-grab'),
      )}
      onPointerDown={placing ? startDragging : undefined}
    >
      <div
        ref={panelRef}
        className={cn(
          'flex w-full flex-col gap-2.5 rounded-panel border border-border-strong',
          'bg-surface-1/95 p-3 shadow-2xl backdrop-blur',
          placing && 'select-none',
        )}
        role="dialog"
        aria-label={t('overlay.title')}
        aria-live="polite"
      >
        {/* Ajustandolo los botones no responden: cualquier clic esta moviendo
            el overlay, asi que se muestran apagados. Estirando la esquina
            vuelven a su color para poder juzgar el tamano de verdad. */}
        <div
          className={cn(
            'flex flex-col gap-2.5 transition-opacity duration-150',
            placing && 'pointer-events-none',
            placing && !resizing && 'opacity-60',
          )}
        >
          <header className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => changePage(-1)}
              disabled={pages.length <= 1}
              aria-label={t('soundboard.previousPage')}
              className="rounded p-1 text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg-default disabled:opacity-40"
            >
              <ChevronLeft className="h-4 w-4" aria-hidden />
            </button>

            <div className="min-w-0 flex-1 text-center">
              <p className="truncate text-sm font-medium text-fg-default">
                {page?.name ?? t('overlay.noPage')}
              </p>
              <p className="font-mono text-[0.625rem] tabular-nums text-fg-subtle">
                {index >= 0 ? index + 1 : 0} / {pages.length}
              </p>
            </div>

            <button
              type="button"
              onClick={() => changePage(1)}
              disabled={pages.length <= 1}
              aria-label={t('soundboard.nextPage')}
              className="rounded p-1 text-fg-muted transition-colors hover:bg-surface-3 hover:text-fg-default disabled:opacity-40"
            >
              <ChevronRight className="h-4 w-4" aria-hidden />
            </button>

            <button
              type="button"
              onClick={close}
              aria-label={t('overlay.close')}
              className="rounded p-1 text-fg-subtle transition-colors hover:bg-surface-3 hover:text-fg-default"
            >
              <X className="h-4 w-4" aria-hidden />
            </button>
          </header>

          {error ? (
            <p className="rounded border border-danger/50 bg-danger-soft px-2 py-1 text-[0.6875rem] text-danger">
              {error}
            </p>
          ) : null}

          <div className="grid grid-cols-3 gap-1.5" role="group" aria-label={t('overlay.buttons')}>
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
                  aria-label={t('slot.label', {
                    number: slot.slotNumber,
                    name: label ?? t('slot.unassigned'),
                    state: '',
                  })}
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
                      'relative font-mono text-[0.625rem] font-semibold',
                      imageSrc ? 'text-white/90 drop-shadow' : 'text-fg-subtle',
                    )}
                  >
                    {slot.slotNumber}
                  </span>
                  <span className="relative min-w-0">
                    <span
                      className={cn(
                        'block truncate text-[0.6875rem] font-medium leading-tight',
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
                          'block font-mono text-[0.5625rem] tabular-nums',
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

          <footer className="flex items-center justify-between gap-2 text-[0.625rem] text-fg-subtle">
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
        </div>

        {/* Estado invisible que confirma si el overlay se considera visible. */}
        <span className="sr-only" data-testid="overlay-visible">
          {visible ? 'visible' : 'oculto'}
        </span>
      </div>

      {/* Fuera del panel: los controles del ajuste son de la ventana, y asi no
          le corren nada de lugar al overlay que se esta previsualizando. */}
      {placing ? <PlacementBar t={t} size={size} faded={resizing} onSave={savePlacement} /> : null}
      {placing ? (
        <ResizeGrip
          label={t('overlay.placementResize')}
          onStart={beginResize}
          onMove={moveResize}
          onEnd={endResize}
        />
      ) : null}
    </div>
  );
}
