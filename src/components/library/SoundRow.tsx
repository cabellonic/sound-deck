import {
  AlertTriangle,
  FolderOpen,
  Image as ImageIcon,
  ImageOff,
  Link2,
  ListPlus,
  MoreHorizontal,
  Pause,
  Pencil,
  Play,
  Trash2,
  Volume1,
} from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/Button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Tooltip,
} from '@/components/ui/primitives';
import { useDragSource } from '@/features/dnd';
import { soundImageSrc } from '@/lib/ipc';
import { cn, formatDuration, volumeToPercent } from '@/lib/utils';
import { CATEGORY_LABELS, type Sound } from '@/types/domain';

export interface SoundRowProps {
  sound: Sound;
  isPreviewing: boolean;
  isPlaying: boolean;
  onTogglePreview: (sound: Sound) => void;
  onAssign: (sound: Sound) => void;
  onRename: (sound: Sound) => void;
  onEditVolume: (sound: Sound) => void;
  onPickImage: (sound: Sound) => void;
  onClearImage: (sound: Sound) => void;
  onDelete: (sound: Sound) => void;
  onReveal: (sound: Sound) => void;
  onOpenSource: (sound: Sound) => void;
}

/** Fila de la biblioteca local (§9). */
export function SoundRow({
  sound,
  isPreviewing,
  isPlaying,
  onTogglePreview,
  onAssign,
  onRename,
  onEditVolume,
  onPickImage,
  onClearImage,
  onDelete,
  onReveal,
  onOpenSource,
}: SoundRowProps) {
  const duration = formatDuration(sound.durationMs);
  const origin = sound.source.type === 'provider' ? sound.source.providerId : 'Importado';

  const imageSrc = soundImageSrc(sound);
  const [brokenSrc, setBrokenSrc] = useState<string | null>(null);
  const showImage = imageSrc !== null && imageSrc !== brokenSrc;

  const { onPointerDown } = useDragSource(
    () => (sound.fileAvailable ? { kind: 'local-sound', soundId: sound.id } : null),
    sound.name,
  );

  return (
    <div
      onPointerDown={onPointerDown}
      className={cn(
        'group flex items-center gap-2 rounded-md border border-border-subtle bg-surface-1 px-2.5 py-2',
        'transition-colors hover:border-border-strong hover:bg-surface-2',
        // La fila se arrastra hacia un boton: la mano abierta lo anticipa.
        sound.fileAvailable && 'cursor-grab active:cursor-grabbing',
        isPlaying && 'border-accent',
        !sound.fileAvailable && 'border-danger/50',
      )}
    >
      <Button
        size="icon"
        variant="ghost"
        onClick={() => onTogglePreview(sound)}
        disabled={!sound.fileAvailable}
        aria-label={isPreviewing ? `Detener ${sound.name}` : `Previsualizar ${sound.name}`}
        className={cn('shrink-0', isPreviewing && 'text-accent')}
      >
        {isPreviewing ? (
          <Pause className="h-4 w-4" aria-hidden />
        ) : (
          <Play className="h-4 w-4" aria-hidden />
        )}
      </Button>

      {showImage ? (
        <img
          src={imageSrc}
          alt=""
          aria-hidden
          draggable={false}
          onError={() => setBrokenSrc(imageSrc)}
          className="h-8 w-8 shrink-0 rounded object-cover"
        />
      ) : null}

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <p className="truncate text-sm font-medium text-fg-default" title={sound.name}>
            {sound.name}
          </p>
          {!sound.fileAvailable ? (
            <Tooltip content="El archivo ya no esta en la carpeta de la aplicacion">
              <AlertTriangle
                className="h-3.5 w-3.5 shrink-0 text-danger"
                aria-label="Archivo no disponible"
              />
            </Tooltip>
          ) : null}
          {sound.customVolume !== null ? (
            <Tooltip
              content={`Volumen propio: ${volumeToPercent(sound.customVolume)}%, sin seguir el general`}
            >
              <Volume1 className="h-3.5 w-3.5 shrink-0 text-fg-subtle" aria-hidden />
            </Tooltip>
          ) : null}
        </div>

        <div className="mt-0.5 flex items-center gap-2 text-[11px] text-fg-subtle">
          {duration ? <span className="font-mono tabular-nums">{duration}</span> : null}
          <span className="truncate">{origin}</span>
          {sound.normalizedCategory !== 'uncategorized' ? (
            <span className="truncate">{CATEGORY_LABELS[sound.normalizedCategory]}</span>
          ) : null}
          {sound.assignedSlotCount > 0 ? (
            <span className="text-accent">
              En {sound.assignedSlotCount} {sound.assignedSlotCount === 1 ? 'boton' : 'botones'}
            </span>
          ) : null}
        </div>
      </div>

      <Tooltip content="Asignar a un boton">
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onAssign(sound)}
          disabled={!sound.fileAvailable}
          aria-label={`Asignar ${sound.name} a un boton`}
          className="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        >
          <ListPlus className="h-4 w-4" aria-hidden />
        </Button>
      </Tooltip>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            size="icon"
            variant="ghost"
            aria-label={`Acciones para ${sound.name}`}
            className="shrink-0"
          >
            <MoreHorizontal className="h-4 w-4" aria-hidden />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuLabel>{sound.name}</DropdownMenuLabel>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={() => onAssign(sound)}>
            <ListPlus className="h-3.5 w-3.5" aria-hidden />
            Asignar a...
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onRename(sound)}>
            <Pencil className="h-3.5 w-3.5" aria-hidden />
            Renombrar
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onEditVolume(sound)}>
            <Volume1 className="h-3.5 w-3.5" aria-hidden />
            Ajustar volumen
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onPickImage(sound)}>
            <ImageIcon className="h-3.5 w-3.5" aria-hidden />
            {sound.imagePath ? 'Cambiar imagen' : 'Poner imagen'}
          </DropdownMenuItem>
          {sound.imagePath ? (
            <DropdownMenuItem onSelect={() => onClearImage(sound)}>
              <ImageOff className="h-3.5 w-3.5" aria-hidden />
              Quitar imagen
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuItem onSelect={() => onReveal(sound)}>
            <FolderOpen className="h-3.5 w-3.5" aria-hidden />
            Abrir carpeta
          </DropdownMenuItem>
          {sound.sourcePageUrl ? (
            <DropdownMenuItem onSelect={() => onOpenSource(sound)}>
              <Link2 className="h-3.5 w-3.5" aria-hidden />
              Ver origen
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuSeparator />
          <DropdownMenuItem destructive onSelect={() => onDelete(sound)}>
            <Trash2 className="h-3.5 w-3.5" aria-hidden />
            Eliminar de la biblioteca
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
