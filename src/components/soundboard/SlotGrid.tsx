import { useCallback, useRef } from 'react';

import type { DragPayload } from '@/lib/drag';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { SlotNumber, Sound, SoundPage, SoundSlot } from '@/types/domain';

import { SlotButton } from './SlotButton';

export interface SlotGridProps {
  page: SoundPage;
  playingSoundIds: string[];
  /** Progreso 0..1 por numero de slot, para los drops desde Internet. */
  slotDownloads: Record<number, number>;
  onPlay: (slotNumber: SlotNumber) => void;
  onDropPayload: (slotNumber: SlotNumber, payload: DragPayload) => void;
  onClear: (slotNumber: SlotNumber) => void;
  onEditLabel: (slot: SoundSlot) => void;
  onEditVolume: (slot: SoundSlot) => void;
  onPickImage: (sound: Sound) => void;
  onClearImage: (sound: Sound) => void;
  onReveal: (soundId: string) => void;
  onShowDetails: (slot: SoundSlot) => void;
}

/**
 * Rejilla 3x3.
 *
 * Las flechas mueven el foco dentro de la rejilla y Enter o Espacio reproducen
 * (§28). Los numeros 1-9 se manejan a nivel de ventana, no aca, para que
 * funcionen tambien sin foco en un boton.
 */
export function SlotGrid(props: SlotGridProps) {
  const { t } = useTranslation();
  const { page, playingSoundIds, slotDownloads } = props;
  const gridRef = useRef<HTMLDivElement>(null);

  const focusSlot = useCallback((slotNumber: number) => {
    const target = gridRef.current?.querySelector<HTMLButtonElement>(
      `button[data-slot="${slotNumber}"]`,
    );
    target?.focus();
  }, []);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement)) return;

    const current = Number(active.dataset.slot);
    if (!Number.isInteger(current) || current < 1 || current > 9) return;

    const deltas: Record<string, number> = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -3,
      ArrowDown: 3,
    };

    const delta = deltas[event.key];
    if (delta === undefined) return;

    // En los bordes horizontales no saltamos de fila: es lo que espera el usuario.
    if (Math.abs(delta) === 1) {
      const column = (current - 1) % 3;
      if ((delta === -1 && column === 0) || (delta === 1 && column === 2)) {
        event.preventDefault();
        return;
      }
    }

    const next = current + delta;
    if (next < 1 || next > 9) return;

    event.preventDefault();
    focusSlot(next);
  };

  return (
    <div
      ref={gridRef}
      role="group"
      aria-label={t('soundboard.label', { page: page.name })}
      onKeyDown={handleKeyDown}
      className={cn('grid grid-cols-3 gap-2')}
    >
      {page.slots.map((slot) => (
        <SlotButton
          key={`${slot.pageId}-${slot.slotNumber}`}
          slot={slot}
          pageId={page.id}
          isPlaying={Boolean(slot.sound && playingSoundIds.includes(slot.sound.id))}
          downloadProgress={slotDownloads[slot.slotNumber] ?? null}
          onPlay={props.onPlay}
          onDropPayload={props.onDropPayload}
          onClear={props.onClear}
          onEditLabel={props.onEditLabel}
          onEditVolume={props.onEditVolume}
          onPickImage={props.onPickImage}
          onClearImage={props.onClearImage}
          onReveal={props.onReveal}
          onShowDetails={props.onShowDetails}
        />
      ))}
    </div>
  );
}
