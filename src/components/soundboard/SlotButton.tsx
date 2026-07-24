import {
  AlertTriangle,
  FolderOpen,
  Image as ImageIcon,
  ImageOff,
  Info,
  Loader2,
  Pencil,
  Trash2,
  Volume1,
  Volume2,
} from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuSeparator,
  ContextMenuTrigger,
  Tooltip,
} from '@/components/ui/primitives';
import { isClickSuppressed, useDragSource, useDropTarget } from '@/features/dnd';
import type { DragPayload } from '@/lib/drag';
import { soundImageSrc } from '@/lib/ipc';
import { cn, formatDuration, volumeToPercent } from '@/lib/utils';
import type { SlotNumber, Sound, SoundSlot } from '@/types/domain';

export interface SlotButtonProps {
  slot: SoundSlot;
  pageId: string;
  isPlaying: boolean;
  /** Progreso 0..1 mientras se descarga un audio arrastrado desde Internet. */
  downloadProgress?: number | null;
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

export function SlotButton({
  slot,
  pageId,
  isPlaying,
  downloadProgress,
  onPlay,
  onDropPayload,
  onClear,
  onEditLabel,
  onEditVolume,
  onPickImage,
  onClearImage,
  onReveal,
  onShowDetails,
}: SlotButtonProps) {
  const sound = slot.sound;
  const label = slot.customLabel ?? sound?.name ?? null;
  const duration = formatDuration(sound?.durationMs);
  const isDownloading = downloadProgress !== null && downloadProgress !== undefined;
  const isBroken = Boolean(sound && !sound.fileAvailable);
  // Un volumen propio (del boton o del audio) es el que ignora el general.
  const ownVolume = slot.customVolume ?? sound?.customVolume ?? null;

  // La imagen es del audio, no del boton: si se asigna otro audio sin imagen,
  // el boton vuelve a verse sin imagen sin que haya que limpiar nada.
  const imageSrc = soundImageSrc(sound);
  // Guardamos que src fallo, no un booleano: asi asignar otra imagen vuelve a
  // intentarlo solo, sin necesidad de resetear nada.
  const [brokenSrc, setBrokenSrc] = useState<string | null>(null);
  const showImage = imageSrc !== null && imageSrc !== brokenSrc;

  const { onPointerDown } = useDragSource(
    () => (sound && !isDownloading ? { kind: 'slot', pageId, slotNumber: slot.slotNumber } : null),
    label ?? `Boton ${slot.slotNumber}`,
  );

  const { dropProps, isOver, isDragging } = useDropTarget(
    `slot:${pageId}:${slot.slotNumber}`,
    (payload) => onDropPayload(slot.slotNumber, payload),
  );

  const handleClick = () => {
    // Al soltar un arrastre el navegador emite un clic: no debe reproducir.
    if (isClickSuppressed()) return;
    onPlay(slot.slotNumber);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onPlay(slot.slotNumber);
    }
  };

  const describedState = isDownloading
    ? 'Descargando'
    : isBroken
      ? 'Archivo no disponible'
      : isPlaying
        ? 'Reproduciendo'
        : label
          ? 'Asignado'
          : 'Vacio';

  const button = (
    <button
      type="button"
      onPointerDown={onPointerDown}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      disabled={isDownloading}
      aria-label={`Boton ${slot.slotNumber}. ${label ?? 'Sin asignar'}. ${describedState}.`}
      data-slot={slot.slotNumber}
      className={cn(
        'group relative flex aspect-square w-full flex-col justify-between overflow-hidden rounded-lg',
        'border p-2.5 text-left transition-colors',
        label
          ? 'border-border-strong bg-surface-2 hover:bg-surface-3'
          : 'border-dashed border-border-subtle bg-surface-1 hover:border-border-strong',
        isPlaying && 'border-accent bg-accent-soft',
        isBroken && 'border-danger/60',
        // Mientras hay algo en el aire, todos los slots se insinuan como destino.
        isDragging && !isOver && 'border-border-strong',
        isOver && 'border-accent bg-accent-soft ring-2 ring-accent',
        isDownloading && 'cursor-progress',
      )}
    >
      {showImage ? (
        <>
          <img
            src={imageSrc}
            alt=""
            aria-hidden
            draggable={false}
            onError={() => setBrokenSrc(imageSrc)}
            className="pointer-events-none absolute inset-0 h-full w-full object-cover"
          />
          {/* Velo hacia abajo: la imagen se ve entera arriba y el nombre sigue
              siendo legible sobre cualquier foto, clara u oscura. */}
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/85 via-black/35 to-black/10"
          />
        </>
      ) : null}

      <div className="relative flex w-full items-start justify-between gap-1">
        <span
          className={cn(
            'font-mono text-xs font-semibold tabular-nums',
            showImage
              ? 'text-white/90 drop-shadow'
              : isPlaying
                ? 'text-accent-strong'
                : 'text-fg-subtle',
          )}
        >
          {slot.slotNumber}
        </span>

        <span className="flex items-center gap-1">
          {ownVolume !== null && label ? (
            <Volume1
              className={cn('h-3 w-3', showImage ? 'text-white/90 drop-shadow' : 'text-fg-subtle')}
              aria-hidden
            />
          ) : null}
          {isBroken ? <AlertTriangle className="h-3.5 w-3.5 text-danger" aria-hidden /> : null}
          {isPlaying ? (
            <Volume2
              className={cn(
                'h-3.5 w-3.5',
                showImage ? 'text-white drop-shadow' : 'text-accent-strong',
              )}
              aria-hidden
            />
          ) : null}
          {isDownloading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin text-accent" aria-hidden />
          ) : null}
        </span>
      </div>

      <div className="relative min-w-0">
        <p
          className={cn(
            'truncate text-sm font-medium leading-tight',
            showImage
              ? 'text-white drop-shadow-[0_1px_2px_rgba(0,0,0,0.9)]'
              : label
                ? 'text-fg-default'
                : 'text-fg-subtle',
          )}
        >
          {label ?? 'Vacio'}
        </p>
        {duration && label ? (
          <p
            className={cn(
              'mt-0.5 font-mono text-[11px] tabular-nums',
              showImage ? 'text-white/80 drop-shadow' : 'text-fg-subtle',
            )}
          >
            {duration}
          </p>
        ) : null}
        {isBroken ? <p className="mt-0.5 text-[11px] text-danger">Archivo faltante</p> : null}
      </div>

      {isDownloading ? (
        <div
          className="absolute inset-x-0 bottom-0 h-1 bg-surface-3"
          role="progressbar"
          aria-valuenow={Math.round((downloadProgress ?? 0) * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label="Progreso de descarga"
        >
          <div
            className="h-full bg-accent transition-[width]"
            style={{ width: `${Math.round((downloadProgress ?? 0) * 100)}%` }}
          />
        </div>
      ) : null}
    </button>
  );

  return (
    <div {...dropProps} className="relative">
      {sound ? (
        <ContextMenu>
          {/* Clic izquierdo reproduce; clic derecho abre el menu (§7). */}
          <ContextMenuTrigger asChild>
            <div>
              <Tooltip content={sound.name}>{button}</Tooltip>
            </div>
          </ContextMenuTrigger>

          <ContextMenuContent>
            <ContextMenuLabel>{sound.name}</ContextMenuLabel>
            <ContextMenuSeparator />
            <ContextMenuItem onSelect={() => onEditLabel(slot)}>
              <Pencil className="h-3.5 w-3.5" aria-hidden />
              Editar nombre visible
            </ContextMenuItem>
            <ContextMenuItem onSelect={() => onEditVolume(slot)}>
              <Volume1 className="h-3.5 w-3.5" aria-hidden />
              Volumen del boton
              {slot.customVolume !== null ? (
                <span className="ml-auto font-mono text-xs text-fg-subtle">
                  {volumeToPercent(slot.customVolume)}%
                </span>
              ) : null}
            </ContextMenuItem>
            <ContextMenuItem onSelect={() => onPickImage(sound)}>
              <ImageIcon className="h-3.5 w-3.5" aria-hidden />
              {sound.imagePath ? 'Cambiar imagen del audio' : 'Poner imagen al audio'}
            </ContextMenuItem>
            {sound.imagePath ? (
              <ContextMenuItem onSelect={() => onClearImage(sound)}>
                <ImageOff className="h-3.5 w-3.5" aria-hidden />
                Quitar imagen
              </ContextMenuItem>
            ) : null}
            <ContextMenuItem onSelect={() => onShowDetails(slot)}>
              <Info className="h-3.5 w-3.5" aria-hidden />
              Ver metadata
            </ContextMenuItem>
            <ContextMenuItem onSelect={() => onReveal(sound.id)}>
              <FolderOpen className="h-3.5 w-3.5" aria-hidden />
              Abrir ubicacion del archivo
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem destructive onSelect={() => onClear(slot.slotNumber)}>
              <Trash2 className="h-3.5 w-3.5" aria-hidden />
              Quitar del boton
            </ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
      ) : (
        button
      )}
    </div>
  );
}
