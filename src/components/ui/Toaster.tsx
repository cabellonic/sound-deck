import { AlertTriangle, CheckCircle2, Info, X, XCircle } from 'lucide-react';
import { useEffect } from 'react';

import type { NoticeLevel } from '@/lib/events';
import { cn } from '@/lib/utils';
import { useTranslation } from '@/i18n/useTranslation';
import { useUiStore, type Toast } from '@/stores/useUiStore';

const ICONS: Record<NoticeLevel, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

const TONES: Record<NoticeLevel, string> = {
  info: 'text-fg-muted',
  success: 'text-success',
  warning: 'text-warning',
  error: 'text-danger',
};

/** Los errores se quedan hasta que el usuario los cierra. */
const DURATIONS: Record<NoticeLevel, number | null> = {
  info: 4000,
  success: 3000,
  warning: 6000,
  error: null,
};

function ToastItem({ toast }: { toast: Toast }) {
  const { t } = useTranslation();
  const dismiss = useUiStore((state) => state.dismissToast);
  const Icon = ICONS[toast.level];

  useEffect(() => {
    const duration = DURATIONS[toast.level];
    if (duration === null) return;

    const timer = window.setTimeout(() => dismiss(toast.id), duration);
    return () => window.clearTimeout(timer);
  }, [toast.id, toast.level, dismiss]);

  return (
    <div
      className={cn(
        'animate-in-fast flex w-80 items-start gap-2.5 rounded-md border border-border-subtle',
        'bg-surface-2 px-3 py-2.5 shadow-xl',
      )}
    >
      <Icon className={cn('mt-0.5 h-4 w-4 shrink-0', TONES[toast.level])} aria-hidden />
      <p className="flex-1 text-sm leading-snug text-fg-default">{toast.message}</p>
      <button
        type="button"
        onClick={() => dismiss(toast.id)}
        aria-label={t('toast.closeNotice')}
        className="rounded p-0.5 text-fg-subtle transition-colors hover:bg-surface-3 hover:text-fg-default"
      >
        <X className="h-3.5 w-3.5" aria-hidden />
      </button>
    </div>
  );
}

/**
 * Avisos de la aplicacion (§33).
 *
 * `aria-live="polite"` hace que un lector de pantalla anuncie los estados
 * importantes sin interrumpir lo que el usuario este haciendo (§28).
 */
export function Toaster() {
  const toasts = useUiStore((state) => state.toasts);

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[100] flex flex-col-reverse gap-2"
      role="status"
      aria-live="polite"
    >
      {toasts.map((toast) => (
        <div key={toast.id} className="pointer-events-auto">
          <ToastItem toast={toast} />
        </div>
      ))}
    </div>
  );
}
