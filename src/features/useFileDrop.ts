/**
 * Archivos soltados desde el explorador del sistema (§10).
 *
 * Con `dragDropEnabled: true` el webview no emite eventos `drop` de HTML5, pero
 * Tauri nos entrega algo mejor: las **rutas absolutas** reales. Un `drop` de
 * HTML5 solo daria objetos `File` sin ruta, y leer su contenido en el frontend
 * para mandarlo por IPC seria justo lo que §31 prohibe.
 *
 * Como no hay eventos de DOM, el destino se resuelve por posicion: se mira que
 * elemento hay bajo el cursor y se leen sus `data-*`. Hay tres destinos, en
 * orden de prioridad: un boton de la botonera, una fila de la biblioteca
 * (solo imagenes) y, si no, la biblioteca entera.
 */
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useEffect, useRef, useState } from 'react';

import type { SlotNumber } from '@/types/domain';

/** Fila de la biblioteca que puede recibir una imagen. */
export type SoundDropTarget =
  | { kind: 'local'; key: string; soundId: string; name: string }
  | {
      kind: 'remote';
      key: string;
      providerId: string;
      remoteId: string;
      name: string;
      /** Id local si el resultado ya se descargo. */
      savedSoundId: string | null;
    };

export interface FileDropHandlers {
  /** Se soltaron archivos sobre un boton concreto. */
  onDropOnSlot: (slotNumber: SlotNumber, paths: string[]) => void;
  /** Se solto una imagen sobre una fila de la biblioteca. */
  onDropImageOnSound: (target: SoundDropTarget, path: string) => void;
  /** Se soltaron archivos en cualquier otro lugar de la ventana. */
  onDropOnLibrary: (paths: string[]) => void;
  /**
   * Si la ruta es una imagen. Lo decide quien usa el hook, que conoce las
   * extensiones que acepta el backend; mientras esa lista no llegue conviene
   * devolver `false` y que el archivo caiga en la importacion normal.
   */
  isImagePath: (path: string) => boolean;
}

/** Slot que hay bajo un punto de la pantalla, si es que hay alguno. */
function slotAt(x: number, y: number): SlotNumber | null {
  const element = document.elementFromPoint(x, y);
  const slot = element?.closest<HTMLElement>('[data-slot]');
  const value = Number(slot?.dataset.slot);
  return Number.isInteger(value) && value >= 1 && value <= 9 ? (value as SlotNumber) : null;
}

/** Fila de la biblioteca que hay bajo un punto de la pantalla. */
export function soundAt(x: number, y: number): SoundDropTarget | null {
  const element = document.elementFromPoint(x, y);
  const row = element?.closest<HTMLElement>('[data-sound-drop]');
  if (!row) return null;

  const { soundDrop, soundId, soundName, providerId, remoteId, savedSoundId } = row.dataset;
  const name = soundName ?? '';

  if (soundDrop === 'local' && soundId) {
    return { kind: 'local', key: `local:${soundId}`, soundId, name };
  }

  if (soundDrop === 'remote' && providerId && remoteId) {
    return {
      kind: 'remote',
      key: `remote:${providerId}:${remoteId}`,
      providerId,
      remoteId,
      name,
      savedSoundId: savedSoundId ?? null,
    };
  }

  return null;
}

/**
 * Escucha los archivos que se sueltan sobre la ventana.
 *
 * Devuelve que hay sobrevolando y donde, para poder resaltar el destino.
 */
export function useFileDrop({
  onDropOnSlot,
  onDropImageOnSound,
  onDropOnLibrary,
  isImagePath,
}: FileDropHandlers): {
  isOver: boolean;
  hoveredSlot: SlotNumber | null;
  /** Es una cadena y no un objeto para no renderizar en cada pixel de movimiento. */
  hoveredSoundKey: string | null;
} {
  const [isOver, setIsOver] = useState(false);
  const [hoveredSlot, setHoveredSlot] = useState<SlotNumber | null>(null);
  const [hoveredSoundKey, setHoveredSoundKey] = useState<string | null>(null);

  // Las rutas llegan en `enter` y en `drop`, pero no en `over`. Guardarlas
  // permite saber, mientras se arrastra, si esto va a poder asignarse como
  // imagen o si termina siendo una importacion.
  const draggedPaths = useRef<string[]>([]);

  // Resuscribir el listener de Tauri cada vez que cambia un handler es como se
  // pierde un drop justo en el medio.
  const handlers = useRef({ onDropOnSlot, onDropImageOnSound, onDropOnLibrary, isImagePath });
  handlers.current = { onDropOnSlot, onDropImageOnSound, onDropOnLibrary, isImagePath };

  useEffect(() => {
    // Tauri informa la posicion en pixeles fisicos; el DOM trabaja en pixeles
    // CSS. Sin esta division, en pantallas con escalado el destino seria otro.
    const toCssPixels = (position: { x: number; y: number }) => ({
      x: position.x / window.devicePixelRatio,
      y: position.y / window.devicePixelRatio,
    });

    const isSingleImage = (paths: string[]) => {
      const [first] = paths;
      return paths.length === 1 && first !== undefined && handlers.current.isImagePath(first);
    };

    const clear = () => {
      draggedPaths.current = [];
      setIsOver(false);
      setHoveredSlot(null);
      setHoveredSoundKey(null);
    };

    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;

      if (payload.type === 'enter') {
        draggedPaths.current = payload.paths;
        setIsOver(true);
        return;
      }

      if (payload.type === 'over') {
        const { x, y } = toCssPixels(payload.position);
        const slot = slotAt(x, y);
        setIsOver(true);
        setHoveredSlot(slot);
        // Un boton gana sobre una fila: es el destino mas especifico.
        setHoveredSoundKey(
          slot === null && isSingleImage(draggedPaths.current)
            ? (soundAt(x, y)?.key ?? null)
            : null,
        );
        return;
      }

      if (payload.type === 'drop') {
        clear();
        if (payload.paths.length === 0) return;

        const { x, y } = toCssPixels(payload.position);
        const slot = slotAt(x, y);

        if (slot !== null) {
          handlers.current.onDropOnSlot(slot, payload.paths);
          return;
        }

        const [image] = payload.paths;
        const sound = isSingleImage(payload.paths) ? soundAt(x, y) : null;
        if (sound && image) {
          handlers.current.onDropImageOnSound(sound, image);
          return;
        }

        handlers.current.onDropOnLibrary(payload.paths);
        return;
      }

      clear();
    });

    return () => {
      void unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  return { isOver, hoveredSlot, hoveredSoundKey };
}
