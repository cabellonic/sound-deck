import { useEffect } from 'react';

import { isEditableTarget, slotFromKey } from '@/lib/utils';
import type { SlotNumber } from '@/types/domain';

export interface SlotKeysOptions {
  enabled: boolean;
  /** Si repetir al mantener la tecla presionada (§16). */
  allowRepeat?: boolean;
  onSlot: (slotNumber: SlotNumber) => void;
  onPrevPage?: () => void;
  onNextPage?: () => void;
  onEscape?: () => void;
}

/**
 * Teclas 1-9 y navegacion de paginas a nivel de ventana.
 *
 * Nunca dispara mientras el foco esta en un campo editable (§28). Escape si
 * llega siempre, para poder cerrar el overlay desde cualquier lado.
 */
export function useSlotKeys({
  enabled,
  allowRepeat = false,
  onSlot,
  onPrevPage,
  onNextPage,
  onEscape,
}: SlotKeysOptions): void {
  useEffect(() => {
    if (!enabled) return;

    const handler = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && onEscape) {
        event.preventDefault();
        event.stopPropagation();
        onEscape();
        return;
      }

      // Un campo de texto tiene prioridad sobre los atajos, salvo Escape.
      if (isEditableTarget(event.target)) return;

      // Las combinaciones con modificadores pertenecen a otros atajos.
      if (event.ctrlKey || event.altKey || event.metaKey) return;

      if (event.repeat && !allowRepeat) return;

      if (event.key === 'PageUp' && onPrevPage) {
        event.preventDefault();
        event.stopPropagation();
        onPrevPage();
        return;
      }

      if (event.key === 'PageDown' && onNextPage) {
        event.preventDefault();
        event.stopPropagation();
        onNextPage();
        return;
      }

      const slot = slotFromKey(event.code, event.key);
      if (slot === null) return;

      // `preventDefault` + `stopPropagation`: la pulsacion muere aca (§16).
      event.preventDefault();
      event.stopPropagation();
      onSlot(slot as SlotNumber);
    };

    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  }, [enabled, allowRepeat, onSlot, onPrevPage, onNextPage, onEscape]);
}
