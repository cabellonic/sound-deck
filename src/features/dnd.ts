/**
 * Drag and drop basado en eventos de puntero.
 *
 * ## Por que no usamos la API HTML5
 *
 * La ventana tiene `dragDropEnabled: true` para que Tauri nos entregue las
 * rutas reales de los archivos que se sueltan desde el explorador. La
 * contrapartida, documentada por Tauri, es que **eso desactiva el drag and drop
 * de HTML5 en Windows**. Como necesitamos las dos cosas, el arrastre interno se
 * resuelve con `pointerdown` / `pointermove` / `pointerup`, que no dependen de
 * esa API y ademas funcionan igual con mouse, trackpad y pantalla tactil.
 */
import { useCallback, useEffect, useRef } from 'react';
import { create } from 'zustand';

import type { DragPayload } from '@/lib/drag';

/** Pixeles que hay que mover antes de considerar que es un arrastre y no un clic. */
const DRAG_THRESHOLD_PX = 6;

/** Ventana en la que se ignora el clic que sigue a un arrastre. */
const CLICK_SUPPRESSION_MS = 300;

export interface DragPosition {
  x: number;
  y: number;
}

interface DragState {
  payload: DragPayload | null;
  /** Texto que se muestra en el fantasma que sigue al cursor. */
  label: string;
  position: DragPosition | null;
  /** Id del destino que esta debajo del cursor, si es valido. */
  hoverTargetId: string | null;
  begin: (payload: DragPayload, label: string, position: DragPosition) => void;
  update: (position: DragPosition, hoverTargetId: string | null) => void;
  finish: () => void;
}

export const useDragStore = create<DragState>((set) => ({
  payload: null,
  label: '',
  position: null,
  hoverTargetId: null,
  begin: (payload, label, position) => set({ payload, label, position, hoverTargetId: null }),
  update: (position, hoverTargetId) => set({ position, hoverTargetId }),
  finish: () => set({ payload: null, label: '', position: null, hoverTargetId: null }),
}));

/** Destinos registrados, por id. Vive fuera de React: no dispara renders. */
const dropTargets = new Map<string, (payload: DragPayload) => void>();

/** Marca temporal hasta la que se ignoran los clics, tras soltar un arrastre. */
let suppressClicksUntil = 0;

/** Si el clic actual es el coletazo de un arrastre recien terminado. */
export function isClickSuppressed(): boolean {
  return Date.now() < suppressClicksUntil;
}

/** Resuelve el destino que hay bajo un punto de la pantalla. */
function resolveTargetAt(position: DragPosition): string | null {
  const element = document.elementFromPoint(position.x, position.y);
  const target = element?.closest<HTMLElement>('[data-drop-id]');
  const id = target?.dataset.dropId ?? null;
  return id !== null && dropTargets.has(id) ? id : null;
}

/**
 * Convierte a un elemento en origen de arrastre.
 *
 * `getPayload` se evalua al empezar el arrastre, no al montar, para que siempre
 * refleje el estado actual de la fila o del slot.
 */
export function useDragSource(getPayload: () => DragPayload | null, label: string) {
  const payloadRef = useRef(getPayload);
  payloadRef.current = getPayload;

  const labelRef = useRef(label);
  labelRef.current = label;

  const onPointerDown = useCallback((event: React.PointerEvent<HTMLElement>) => {
    // Solo boton principal: el derecho abre el menu contextual.
    if (event.button !== 0) return;

    const payload = payloadRef.current();
    if (!payload) return;

    const origin = { x: event.clientX, y: event.clientY };
    const store = useDragStore.getState();
    let dragging = false;

    const handleMove = (moveEvent: PointerEvent) => {
      const position = { x: moveEvent.clientX, y: moveEvent.clientY };

      if (!dragging) {
        const distance = Math.hypot(position.x - origin.x, position.y - origin.y);
        if (distance < DRAG_THRESHOLD_PX) return;

        dragging = true;
        store.begin(payload, labelRef.current, position);
        // Sin esto, arrastrar selecciona texto por toda la interfaz.
        document.body.style.userSelect = 'none';
      }

      store.update(position, resolveTargetAt(position));
    };

    const handleUp = (upEvent: PointerEvent) => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
      window.removeEventListener('pointercancel', handleCancel);

      if (!dragging) return;

      document.body.style.userSelect = '';
      const targetId = resolveTargetAt({ x: upEvent.clientX, y: upEvent.clientY });
      useDragStore.getState().finish();

      // El clic que el navegador emite a continuacion no debe reproducir nada.
      suppressClicksUntil = Date.now() + CLICK_SUPPRESSION_MS;

      if (targetId) dropTargets.get(targetId)?.(payload);
    };

    const handleCancel = () => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
      window.removeEventListener('pointercancel', handleCancel);

      if (dragging) {
        document.body.style.userSelect = '';
        useDragStore.getState().finish();
      }
    };

    window.addEventListener('pointermove', handleMove);
    window.addEventListener('pointerup', handleUp);
    window.addEventListener('pointercancel', handleCancel);
  }, []);

  return { onPointerDown };
}

/**
 * Convierte a un elemento en destino de arrastre.
 *
 * Devuelve las props que hay que aplicar y si el cursor esta encima ahora.
 */
export function useDropTarget(id: string, onDrop: (payload: DragPayload) => void) {
  const handlerRef = useRef(onDrop);
  handlerRef.current = onDrop;

  useEffect(() => {
    dropTargets.set(id, (payload) => handlerRef.current(payload));
    return () => {
      dropTargets.delete(id);
    };
  }, [id]);

  const isOver = useDragStore((state) => state.hoverTargetId === id);
  const isDragging = useDragStore((state) => state.payload !== null);

  return { dropProps: { 'data-drop-id': id }, isOver, isDragging };
}

/** Payload que se esta arrastrando ahora, para decidir si un destino aplica. */
export function useActiveDragPayload(): DragPayload | null {
  return useDragStore((state) => state.payload);
}
