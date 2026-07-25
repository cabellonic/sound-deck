import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AudioLines, Upload, Volume2, Wifi } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/Button';
import { Dialog, DialogContent, DialogFooter, DialogHeader } from '@/components/ui/primitives';
import { queryKeys } from '@/features/queryKeys';
import { useTranslation } from '@/i18n/useTranslation';
import { formatAccelerator } from '@/lib/utils';
import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { useUiStore } from '@/stores/useUiStore';

export interface OnboardingProps {
  onImport: () => void;
  onOpenSettings: () => void;
}

/**
 * Introduccion breve del primer arranque (§32). Siempre se puede omitir.
 */
export function Onboarding({ onImport, onOpenSettings }: OnboardingProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(true);
  const queryClient = useQueryClient();
  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: ipc.getSettings });
  const overlayAccelerator = formatAccelerator(
    settings.data?.shortcuts.bindings.find((binding) => binding.action === 'toggle_overlay')
      ?.accelerator ?? 'Alt+Home',
    t,
  );
  const pushToast = useUiStore((state) => state.pushToast);

  const complete = useMutation({
    mutationFn: ipc.completeOnboarding,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      void queryClient.invalidateQueries({ queryKey: queryKeys.appState });
    },
  });

  const close = () => {
    setOpen(false);
    complete.mutate();
  };

  const testDevice = () => {
    void ipc
      .testAudioDevice()
      .then(() => pushToast('success', t('onboarding.testedDevice')))
      .catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  const steps = [
    {
      icon: Upload,
      title: t('onboarding.importTitle'),
      description: t('onboarding.importDescription'),
      action: (
        <Button size="sm" onClick={onImport}>
          {t('library.importSounds')}
        </Button>
      ),
    },
    {
      icon: Volume2,
      title: t('onboarding.outputTitle'),
      description: t('onboarding.outputDescription'),
      action: (
        <Button size="sm" variant="secondary" onClick={testDevice}>
          {t('onboarding.testAudio')}
        </Button>
      ),
    },
    {
      icon: AudioLines,
      title: t('onboarding.boardTitle'),
      description: t('onboarding.boardDescription', { accelerator: overlayAccelerator }),
      action: null,
    },
    {
      icon: Wifi,
      title: t('onboarding.providersTitle'),
      description: t('onboarding.providersDescription'),
      action: (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            close();
            onOpenSettings();
          }}
        >
          {t('onboarding.configureProvider')}
        </Button>
      ),
    },
  ];

  return (
    <Dialog open={open} onOpenChange={(next) => (next ? setOpen(true) : close())}>
      <DialogContent className="max-w-lg">
        <DialogHeader title={t('onboarding.title')} description={t('onboarding.description')} />

        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4">
          {steps.map(({ icon: Icon, title, description, action }) => (
            <div
              key={title}
              className="flex items-start gap-3 rounded-md border border-border-subtle bg-surface-2 p-3"
            >
              <Icon className="mt-0.5 h-4 w-4 shrink-0 text-accent" aria-hidden />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-fg-default">{title}</p>
                <p className="mt-0.5 text-xs leading-relaxed text-fg-muted">{description}</p>
                {action ? <div className="mt-2">{action}</div> : null}
              </div>
            </div>
          ))}
        </div>

        <DialogFooter>
          <Button variant="primary" onClick={close}>
            {t('onboarding.start')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
