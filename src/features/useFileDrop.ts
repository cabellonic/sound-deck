/**
 * Archivos soltados desde el explorador del sistema (§10).
 *
 * Con `dragDropEnabled: true` el webview no emite eventos `drop` de HTML5, pero
 * Tauri nos entrega algo mejor: las **rutas absolutas** reales. Un `drop` de
 * HTML5 solo daria objetos `File` sin ruta, y leer su contenido en el frontend
 * para mandarlo por IPC seria justo lo que §31 prohibe.
 */
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useEffect, useState } from 'react';

import type { SlotNumber } from '@/types/domain';

export interface FileDropHandlers {
  /** Se soltaron archivos sobre un boton concreto. */
  onDropOnSlot: (slotNumber: SlotNumber, paths: string[]) => void;
  /** Se soltaron archivos en cualquier otro lugar de la ventana. */
  onDropOnLibrary: (paths: string[]) => void;
}

/** Slot que hay bajo un punto de la pantalla, si es que hay alguno. */
function slotAt(x: number, y: number): SlotNumber | null {
  const element = document.elementFromPoint(x, y);
  const slot = element?.closest<HTMLElement>('[data-slot]');
  const value = Number(slot?.dataset.slot);
  return Number.isInteger(value) && value >= 1 && value <= 9 ? (value as SlotNumber) : null;
}

/**
 * Escucha los archivos que se sueltan sobre la ventana.
 *
 * Devuelve si hay algo sobrevolando, para poder resaltar la zona de destino.
 */
export function useFileDrop({ onDropOnSlot, onDropOnLibrary }: FileDropHandlers): {
  isOver: boolean;
  hoveredSlot: SlotNumber | null;
} {
  const [isOver, setIsOver] = useState(false);
  const [hoveredSlot, setHoveredSlot] = useState<SlotNumber | null>(null);

  useEffect(() => {
    // Tauri informa la posicion en pixeles fisicos; el DOM trabaja en pixeles
    // CSS. Sin esta division, en pantallas con escalado el destino seria otro.
    const toCssPixels = (position: { x: number; y: number }) => ({
      x: position.x / window.devicePixelRatio,
      y: position.y / window.devicePixelRatio,
    });

    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;

      if (payload.type === 'over') {
        const { x, y } = toCssPixels(payload.position);
        setIsOver(true);
        setHoveredSlot(slotAt(x, y));
        return;
      }

      if (payload.type === 'drop') {
        setIsOver(false);
        setHoveredSlot(null);

        if (payload.paths.length === 0) return;

        const { x, y } = toCssPixels(payload.position);
        const slot = slotAt(x, y);

        if (slot !== null) {
          onDropOnSlot(slot, payload.paths);
        } else {
          onDropOnLibrary(payload.paths);
        }
        return;
      }

      setIsOver(false);
      setHoveredSlot(null);
    });

    return () => {
      void unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, [onDropOnSlot, onDropOnLibrary]);

  return { isOver, hoveredSlot };
}
