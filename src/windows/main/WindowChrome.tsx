/**
 * Botones y bordes de la ventana principal, que no tiene marco del sistema.
 *
 * Sacar la decoracion nativa cuesta reponer a mano todo lo que daba: mover,
 * minimizar, maximizar, cerrar y estirar desde los bordes. A cambio, la barra
 * de titulo es parte de la aplicacion y no una franja gris pegada arriba.
 */
import { getCurrentWindow, type Window as TauriWindow } from '@tauri-apps/api/window';
import { Copy, Minus, Square, X } from 'lucide-react';
import { useEffect, useState } from 'react';

import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';

/** La API no exporta el tipo, asi que se lo pedimos al metodo que lo usa. */
type ResizeDirection = Parameters<TauriWindow['startResizeDragging']>[0];

/**
 * Bordes invisibles para estirar la ventana.
 *
 * Los dos sistemas traen lo suyo: Windows conserva el marco grueso aunque no
 * se dibuje, y tao le pone a GTK un hit-test de 5px cuando la ventana no esta
 * decorada. Pero los dos actuan por debajo del webview, que ocupa la ventana
 * entera y se queda con el clic. Estos van en el DOM, asi que reciben el
 * evento siempre y hacen lo mismo en todos lados.
 */
const EDGES: { direction: ResizeDirection; className: string }[] = [
  { direction: 'North', className: 'inset-x-2 top-0 h-1 cursor-ns-resize' },
  { direction: 'South', className: 'inset-x-2 bottom-0 h-1 cursor-ns-resize' },
  { direction: 'West', className: 'inset-y-2 left-0 w-1 cursor-ew-resize' },
  { direction: 'East', className: 'inset-y-2 right-0 w-1 cursor-ew-resize' },
  { direction: 'NorthWest', className: 'left-0 top-0 h-2 w-2 cursor-nwse-resize' },
  { direction: 'NorthEast', className: 'right-0 top-0 h-2 w-2 cursor-nesw-resize' },
  { direction: 'SouthWest', className: 'bottom-0 left-0 h-2 w-2 cursor-nesw-resize' },
  { direction: 'SouthEast', className: 'bottom-0 right-0 h-2 w-2 cursor-nwse-resize' },
];

export function ResizeEdges() {
  return (
    <div className="pointer-events-none fixed inset-0 z-50" aria-hidden>
      {EDGES.map(({ direction, className }) => (
        <div
          key={direction}
          className={cn('pointer-events-auto absolute', className)}
          onPointerDown={(event) => {
            if (event.button !== 0) return;
            void getCurrentWindow()
              .startResizeDragging(direction)
              .catch(() => undefined);
          }}
        />
      ))}
    </div>
  );
}

function ControlButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      title={label}
      className={cn(
        'flex h-8 w-9 items-center justify-center rounded text-fg-muted transition-colors',
        danger
          ? 'hover:bg-danger hover:text-surface-0'
          : 'hover:bg-surface-3 hover:text-fg-default',
      )}
    >
      {children}
    </button>
  );
}

export function WindowControls() {
  const { t } = useTranslation();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const sync = () => {
      void getCurrentWindow()
        .isMaximized()
        .then(setMaximized)
        .catch(() => undefined);
    };

    sync();
    window.addEventListener('resize', sync);
    return () => window.removeEventListener('resize', sync);
  }, []);

  return (
    <div className="flex items-center gap-0.5">
      <ControlButton label={t('app.minimize')} onClick={() => void getCurrentWindow().minimize()}>
        <Minus className="h-4 w-4" aria-hidden />
      </ControlButton>

      <ControlButton
        label={maximized ? t('app.restore') : t('app.maximize')}
        onClick={() => void getCurrentWindow().toggleMaximize()}
      >
        {maximized ? (
          <Copy className="h-3.5 w-3.5 -scale-x-100" aria-hidden />
        ) : (
          <Square className="h-3.5 w-3.5" aria-hidden />
        )}
      </ControlButton>

      {/* Cerrar pasa por el backend, que decide si esconde a la bandeja o
          termina de verdad segun la configuracion. */}
      <ControlButton label={t('app.close')} danger onClick={() => void getCurrentWindow().close()}>
        <X className="h-4 w-4" aria-hidden />
      </ControlButton>
    </div>
  );
}
