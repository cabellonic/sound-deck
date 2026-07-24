import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AudioLines, Upload, Volume2, Wifi } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/Button';
import { Dialog, DialogContent, DialogFooter, DialogHeader } from '@/components/ui/primitives';
import { queryKeys } from '@/features/queryKeys';
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
  const [open, setOpen] = useState(true);
  const queryClient = useQueryClient();
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
      .then(() => pushToast('success', 'Si escuchaste un tono corto, el audio esta funcionando.'))
      .catch((error: unknown) => pushToast('error', errorMessage(error)));
  };

  const steps = [
    {
      icon: Upload,
      title: 'Importa tus audios',
      description: 'MP3, WAV, OGG o FLAC. Se copian a una carpeta propia para que nunca se rompan.',
      action: (
        <Button size="sm" onClick={onImport}>
          Importar audios
        </Button>
      ),
    },
    {
      icon: Volume2,
      title: 'Elegi donde suena',
      description:
        'Para enviarlo a Discord necesitas un dispositivo virtual (VB-Cable o similar) ya instalado.',
      action: (
        <Button size="sm" variant="secondary" onClick={testDevice}>
          Probar el audio
        </Button>
      ),
    },
    {
      icon: AudioLines,
      title: 'Arma tu botonera',
      description:
        'Arrastra audios a los nueve botones. Ctrl + Alt + Espacio abre el overlay sobre cualquier juego y las teclas 1 a 9 los disparan.',
      action: null,
    },
    {
      icon: Wifi,
      title: 'Busca audios online (opcional)',
      description:
        'Activa un proveedor y carga su API key para buscar en Internet desde la pestana correspondiente.',
      action: (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            close();
            onOpenSettings();
          }}
        >
          Configurar proveedor
        </Button>
      ),
    },
  ];

  return (
    <Dialog open={open} onOpenChange={(next) => (next ? setOpen(true) : close())}>
      <DialogContent className="max-w-lg">
        <DialogHeader
          title="Bienvenido a Sound Deck"
          description="Una soundboard local: tus audios viven en tu computadora y funcionan sin conexion."
        />

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
            Empezar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
