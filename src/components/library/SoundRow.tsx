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
import { categoryKey } from '@/i18n';
import { useTranslation } from '@/i18n/useTranslation';
import { soundImageSrc } from '@/lib/ipc';
import { cn, formatDuration, volumeToPercent } from '@/lib/utils';
import type { Sound } from '@/types/domain';

export interface SoundRowProps {
  sound: Sound;
  isPreviewing: boolean;
  isPlaying: boolean;
  /** `true` mientras hay una imagen del sistema sobrevolando esta fila. */
  isImageDropTarget: boolean;
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
  isImageDropTarget,
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
  const { t, tp } = useTranslation();
  const duration = formatDuration(sound.durationMs);
  const origin = sound.source.type === 'provider' ? sound.source.providerId : t('sound.imported');

  const imageSrc = soundImageSrc(sound);
  const [brokenSrc, setBrokenSrc] = useState<string | null>(null);
  const showImage = imageSrc !== null && imageSrc !== brokenSrc;
  const assignment = sound.assignedSlot
    ? t('sound.assignedTo', {
        page: sound.assignedSlot.pageName,
        slot: sound.assignedSlot.slotNumber,
      })
    : sound.assignedSlotCount > 0
      ? tp(sound.assignedSlotCount, 'sound.assignedCount.one', 'sound.assignedCount.many', {
          count: sound.assignedSlotCount,
        })
      : null;

  const { onPointerDown } = useDragSource(
    () => (sound.fileAvailable ? { kind: 'local-sound', soundId: sound.id } : null),
    sound.name,
  );

  return (
    <div
      onPointerDown={onPointerDown}
      // Soltar una imagen del sistema sobre la fila se la asigna al audio. El
      // drop lo resuelve Tauri por posicion, buscando estos atributos (§10).
      data-sound-drop="local"
      data-sound-id={sound.id}
      data-sound-name={sound.name}
      className={cn(
        'group flex items-center gap-2 rounded-md border border-border-subtle bg-surface-1 px-2.5 py-2',
        'transition-colors hover:border-border-strong hover:bg-surface-2',
        // La fila se arrastra hacia un boton: la mano abierta lo anticipa.
        sound.fileAvailable && 'cursor-grab active:cursor-grabbing',
        isPlaying && 'border-accent',
        !sound.fileAvailable && 'border-danger/50',
        isImageDropTarget && 'border-accent ring-2 ring-inset ring-accent',
      )}
    >
      <Button
        size="icon"
        variant="ghost"
        onClick={() => onTogglePreview(sound)}
        disabled={!sound.fileAvailable}
        aria-label={
          isPreviewing
            ? t('sound.stopPreview', { name: sound.name })
            : t('sound.preview', { name: sound.name })
        }
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
          className={cn('h-8 w-8 shrink-0 rounded object-cover', isImageDropTarget && 'opacity-40')}
        />
      ) : isImageDropTarget ? (
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded border border-dashed border-accent">
          <ImageIcon className="h-4 w-4 text-accent" aria-hidden />
        </span>
      ) : null}

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <p className="truncate text-sm font-medium text-fg-default" title={sound.name}>
            {sound.name}
          </p>
          {!sound.fileAvailable ? (
            <Tooltip content={t('sound.missingFile')}>
              <AlertTriangle
                className="h-3.5 w-3.5 shrink-0 text-danger"
                aria-label={t('sound.missingFileLabel')}
              />
            </Tooltip>
          ) : null}
          {sound.customVolume !== null ? (
            <Tooltip
              content={t('sound.customVolume', {
                percent: volumeToPercent(sound.customVolume),
              })}
            >
              <Volume1 className="h-3.5 w-3.5 shrink-0 text-fg-subtle" aria-hidden />
            </Tooltip>
          ) : null}
        </div>

        <div className="mt-0.5 flex items-center gap-2 text-[11px] text-fg-subtle">
          {duration ? <span className="font-mono tabular-nums">{duration}</span> : null}
          <span className="truncate">{origin}</span>
          {sound.normalizedCategory !== 'uncategorized' ? (
            <span className="truncate">{t(categoryKey(sound.normalizedCategory))}</span>
          ) : null}
          {assignment ? (
            <span className="min-w-0 truncate text-accent" title={assignment}>
              {assignment}
            </span>
          ) : null}
        </div>
      </div>

      <Tooltip content={t('sound.assignTo')}>
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onAssign(sound)}
          disabled={!sound.fileAvailable}
          aria-label={t('sound.assignToLabel', { name: sound.name })}
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
            aria-label={t('sound.actions', { name: sound.name })}
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
            {t('sound.assignMenu')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onRename(sound)}>
            <Pencil className="h-3.5 w-3.5" aria-hidden />
            {t('sound.rename')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onEditVolume(sound)}>
            <Volume1 className="h-3.5 w-3.5" aria-hidden />
            {t('sound.editVolume')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onPickImage(sound)}>
            <ImageIcon className="h-3.5 w-3.5" aria-hidden />
            {sound.imagePath ? t('sound.changeImage') : t('sound.setImage')}
          </DropdownMenuItem>
          {sound.imagePath ? (
            <DropdownMenuItem onSelect={() => onClearImage(sound)}>
              <ImageOff className="h-3.5 w-3.5" aria-hidden />
              {t('sound.clearImage')}
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuItem onSelect={() => onReveal(sound)}>
            <FolderOpen className="h-3.5 w-3.5" aria-hidden />
            {t('sound.openFolder')}
          </DropdownMenuItem>
          {sound.sourcePageUrl ? (
            <DropdownMenuItem onSelect={() => onOpenSource(sound)}>
              <Link2 className="h-3.5 w-3.5" aria-hidden />
              {t('sound.viewSource')}
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuSeparator />
          <DropdownMenuItem destructive onSelect={() => onDelete(sound)}>
            <Trash2 className="h-3.5 w-3.5" aria-hidden />
            {t('sound.deleteFromLibrary')}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
