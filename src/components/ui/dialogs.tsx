import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/Button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  Slider,
  Switch,
} from '@/components/ui/primitives';
import { Input } from '@/components/ui/Input';
import { categoryKey } from '@/i18n';
import { useTranslation } from '@/i18n/useTranslation';
import { cn, formatBytes, formatDuration, relativeDate, volumeToPercent } from '@/lib/utils';
import type { Sound } from '@/types/domain';

export interface PromptDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  label: string;
  initialValue: string;
  confirmLabel?: string;
  onConfirm: (value: string) => void;
}

/** Dialogo de texto simple (renombrar pagina, renombrar audio, etiqueta). */
export function PromptDialog({
  open,
  onOpenChange,
  title,
  description,
  label,
  initialValue,
  confirmLabel,
  onConfirm,
}: PromptDialogProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState(initialValue);

  useEffect(() => {
    if (open) setValue(initialValue);
  }, [open, initialValue]);

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onConfirm(trimmed);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader title={title} description={description} />
        <form
          className="px-5 py-4"
          onSubmit={(event) => {
            event.preventDefault();
            submit();
          }}
        >
          <label htmlFor="prompt-value" className="mb-1.5 block text-sm font-medium">
            {label}
          </label>
          <Input
            id="prompt-value"
            value={value}
            onChange={(event) => setValue(event.target.value)}
            autoFocus
          />
        </form>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" onClick={submit} disabled={!value.trim()}>
            {confirmLabel ?? t('common.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  /** Detalle adicional, por ejemplo donde esta usado un audio. */
  details?: React.ReactNode;
  confirmLabel?: string;
  destructive?: boolean;
  onConfirm: () => void;
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  details,
  confirmLabel,
  destructive = true,
  onConfirm,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader title={title} description={description} />
        {details ? <div className="px-5 py-3 text-sm text-fg-muted">{details}</div> : null}
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button
            variant={destructive ? 'danger' : 'primary'}
            onClick={() => {
              onConfirm();
              onOpenChange(false);
            }}
          >
            {confirmLabel ?? t('common.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export interface VolumeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  /** Volumen propio actual. `null` significa que esta linkeado. */
  value: number | null;
  /** Volumen que se aplica mientras esta linkeado. */
  inheritedVolume: number;
  /** Texto del interruptor, p. ej. "Seguir el volumen general". */
  inheritLabel: string;
  /** Que implica dejarlo linkeado, en una linea. */
  inheritHint: string;
  onConfirm: (value: number | null) => void;
}

/**
 * Volumen propio de un audio o de un boton.
 *
 * Los volumenes propios son absolutos: mientras el interruptor esta activado el
 * elemento sigue al nivel de arriba, y al desactivarlo queda fijo en el valor
 * elegido pase lo que pase con ese nivel.
 */
export function VolumeDialog({
  open,
  onOpenChange,
  title,
  description,
  value,
  inheritedVolume,
  inheritLabel,
  inheritHint,
  onConfirm,
}: VolumeDialogProps) {
  const { t } = useTranslation();
  const [linked, setLinked] = useState(value === null);
  const [current, setCurrent] = useState(value ?? inheritedVolume);

  useEffect(() => {
    if (!open) return;
    setLinked(value === null);
    // Al deslinkear, el slider arranca donde el audio ya venia sonando: mover
    // el control es entonces un ajuste, no un salto a un valor arbitrario.
    setCurrent(value ?? inheritedVolume);
  }, [open, value, inheritedVolume]);

  const shown = linked ? inheritedVolume : current;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader title={title} description={description} />
        <div className="px-5 py-5">
          <label className="flex items-start gap-3">
            <Switch checked={linked} onCheckedChange={setLinked} aria-label={inheritLabel} />
            <span className="min-w-0">
              <span className="block text-sm font-medium text-fg-default">{inheritLabel}</span>
              <span className="mt-0.5 block text-xs text-fg-subtle">{inheritHint}</span>
            </span>
          </label>

          <p
            className={cn(
              'mb-3 mt-5 font-mono text-2xl tabular-nums',
              linked ? 'text-fg-subtle' : 'text-fg-default',
            )}
          >
            {volumeToPercent(shown)}%
          </p>
          <Slider
            value={[volumeToPercent(shown)]}
            onValueChange={([next]) => setCurrent((next ?? 0) / 100)}
            disabled={linked}
            max={100}
            step={1}
            aria-label={title}
            className={cn(linked && 'opacity-50')}
          />
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="primary"
            onClick={() => {
              onConfirm(linked ? null : current);
              onOpenChange(false);
            }}
          >
            {t('common.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Metadata completa de un audio (§7 "ver metadata"). */
export function SoundDetailsDialog({
  open,
  onOpenChange,
  sound,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sound: Sound | null;
}) {
  const { t, locale } = useTranslation();
  if (!sound) return null;

  const none = t('common.none');

  /** Traduce una fecha relativa ya descompuesta. */
  const fecha = (iso: string | null): string => {
    const parsed = relativeDate(iso, locale);
    switch (parsed.kind) {
      case 'never':
        return t('date.never');
      case 'invalid':
        return none;
      case 'now':
        return t('date.now');
      case 'absolute':
        return parsed.value;
      default:
        return t(`date.${parsed.kind}`, { value: parsed.value });
    }
  };
  const rows: Array<[string, string]> = [
    [t('details.name'), sound.name],
    [t('details.originalName'), sound.originalName ?? none],
    [t('details.duration'), formatDuration(sound.durationMs) ?? t('common.unknown')],
    [t('details.format'), sound.fileExtension?.toUpperCase() ?? none],
    [t('details.size'), formatBytes(sound.fileSizeBytes)],
    [t('details.category'), t(categoryKey(sound.normalizedCategory))],
    [t('details.tags'), sound.tags.length > 0 ? sound.tags.join(', ') : none],
    [
      t('details.origin'),
      sound.source.type === 'provider'
        ? `${sound.source.providerId} (id ${sound.source.remoteId})`
        : t('details.importedLocally'),
    ],
    [t('details.license'), sound.license ? sound.license.name : none],
    [t('details.attribution'), sound.attribution ?? none],
    [
      t('details.volume'),
      sound.customVolume === null
        ? t('details.followsMaster')
        : `${volumeToPercent(sound.customVolume)}% (propio)`,
    ],
    [t('details.image'), sound.imagePath ? t('details.imageAssigned') : t('details.imageMissing')],
    [t('details.playCount'), String(sound.playCount)],
    [t('details.lastPlayed'), fecha(sound.lastPlayedAt)],
    [t('details.added'), fecha(sound.createdAt)],
    [
      t('details.file'),
      sound.fileAvailable ? t('details.fileAvailable') : t('details.fileMissing'),
    ],
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader title={t('details.title')} description={sound.name} />
        <dl className="min-h-0 flex-1 overflow-y-auto px-5 py-4 text-sm">
          {rows.map(([label, value]) => (
            <div
              key={label}
              className="flex gap-3 border-b border-border-subtle py-1.5 last:border-0"
            >
              <dt className="w-36 shrink-0 text-fg-subtle">{label}</dt>
              <dd className="min-w-0 flex-1 break-words text-fg-default">{value}</dd>
            </div>
          ))}
        </dl>
        <DialogFooter>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            {t('common.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
