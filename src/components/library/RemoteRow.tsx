import { Check, Download, ExternalLink, ListPlus, Loader2, Pause, Play } from 'lucide-react';

import { Button } from '@/components/ui/Button';
import { Tooltip } from '@/components/ui/primitives';
import { useDragSource } from '@/features/dnd';
import { cn, formatBytes, formatDuration } from '@/lib/utils';
import type { DownloadState } from '@/stores/useUiStore';
import { CATEGORY_LABELS, type RemoteSound } from '@/types/domain';

/** Estados visibles de un resultado online (§13). */
export type RemoteItemStatus =
  'idle' | 'loading-preview' | 'previewing' | 'downloading' | 'saved' | 'error' | 'unavailable';

export interface RemoteRowProps {
  item: RemoteSound;
  status: RemoteItemStatus;
  download: DownloadState | undefined;
  errorMessage?: string;
  onTogglePreview: (item: RemoteSound) => void;
  onDownload: (item: RemoteSound) => void;
  onAssign: (item: RemoteSound) => void;
  onOpenSource: (item: RemoteSound) => void;
}

export function RemoteRow({
  item,
  status,
  download,
  errorMessage,
  onTogglePreview,
  onDownload,
  onAssign,
  onOpenSource,
}: RemoteRowProps) {
  const duration = formatDuration(item.durationMs ?? null);
  const canPreview = Boolean(item.previewUrl);
  const isBusy = status === 'downloading' || status === 'loading-preview';

  const progress =
    download && download.totalBytes
      ? Math.min(download.receivedBytes / download.totalBytes, 1)
      : null;

  const { onPointerDown } = useDragSource(
    () =>
      isBusy
        ? null
        : { kind: 'remote-sound', providerId: item.providerId, remoteId: item.remoteId },
    item.title,
  );

  return (
    <div
      onPointerDown={onPointerDown}
      className={cn(
        'group relative flex items-center gap-2 overflow-hidden rounded-md border',
        'border-border-subtle bg-surface-1 px-2.5 py-2 transition-colors',
        'hover:border-border-strong hover:bg-surface-2',
        !isBusy && 'cursor-grab active:cursor-grabbing',
        status === 'saved' && 'border-success/50',
        status === 'error' && 'border-danger/50',
      )}
    >
      <Button
        size="icon"
        variant="ghost"
        onClick={() => onTogglePreview(item)}
        disabled={!canPreview || status === 'downloading'}
        aria-label={
          canPreview
            ? status === 'previewing'
              ? `Detener ${item.title}`
              : `Previsualizar ${item.title}`
            : `${item.title} no ofrece previsualizacion`
        }
        className={cn('shrink-0', status === 'previewing' && 'text-accent')}
      >
        {status === 'loading-preview' ? (
          <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
        ) : status === 'previewing' ? (
          <Pause className="h-4 w-4" aria-hidden />
        ) : (
          <Play className="h-4 w-4" aria-hidden />
        )}
      </Button>

      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-fg-default" title={item.title}>
          {item.title}
        </p>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] text-fg-subtle">
          <span className="capitalize">{item.providerId}</span>
          {duration ? <span className="font-mono tabular-nums">{duration}</span> : null}
          {item.fileSizeBytes ? <span>{formatBytes(item.fileSizeBytes)}</span> : null}
          {item.normalizedCategory ? <span>{CATEGORY_LABELS[item.normalizedCategory]}</span> : null}
          {item.license ? (
            <Tooltip content={item.attribution ?? item.license.name}>
              <span className="rounded bg-surface-3 px-1 py-px">{item.license.code}</span>
            </Tooltip>
          ) : null}
          {item.tags.length > 0 ? (
            <span className="truncate">{item.tags.slice(0, 3).join(', ')}</span>
          ) : null}
        </div>
        {status === 'error' && errorMessage ? (
          <p className="mt-0.5 text-[11px] text-danger">{errorMessage}</p>
        ) : null}
      </div>

      {status === 'saved' ? (
        <span className="flex shrink-0 items-center gap-1 text-[11px] text-success">
          <Check className="h-3.5 w-3.5" aria-hidden />
          Guardado
        </span>
      ) : null}

      {/* Orden: origen, asignar, descargar. La descarga va ultima y siempre
          visible, porque es la accion principal de la fila. */}
      {item.sourcePageUrl ? (
        <Tooltip content="Abrir pagina de origen">
          <Button
            size="icon"
            variant="ghost"
            onClick={() => onOpenSource(item)}
            aria-label={`Abrir la pagina de origen de ${item.title}`}
            className="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
          >
            <ExternalLink className="h-4 w-4" aria-hidden />
          </Button>
        </Tooltip>
      ) : null}

      <Tooltip content="Descargar y asignar a un boton">
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onAssign(item)}
          disabled={isBusy}
          aria-label={`Asignar ${item.title} a un boton`}
          className="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        >
          <ListPlus className="h-4 w-4" aria-hidden />
        </Button>
      </Tooltip>

      <Tooltip content="Descargar y guardar en la biblioteca">
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onDownload(item)}
          disabled={isBusy}
          aria-label={`Descargar ${item.title}`}
          className="shrink-0"
        >
          {status === 'downloading' ? (
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
          ) : (
            <Download className="h-4 w-4" aria-hidden />
          )}
        </Button>
      </Tooltip>

      {progress !== null ? (
        <div
          className="absolute inset-x-0 bottom-0 h-0.5 bg-surface-3"
          role="progressbar"
          aria-valuenow={Math.round(progress * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={`Descargando ${item.title}`}
        >
          <div
            className="h-full bg-accent transition-[width]"
            style={{ width: `${Math.round(progress * 100)}%` }}
          />
        </div>
      ) : null}
    </div>
  );
}
