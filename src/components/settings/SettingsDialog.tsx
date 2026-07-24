import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import {
  AlertTriangle,
  AudioLines,
  Cable,
  FolderOpen,
  Heart,
  Keyboard,
  Library,
  Loader2,
  RotateCcw,
  Settings2,
  Sliders,
  Volume2,
  Wifi,
} from 'lucide-react';
import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/Button';
import { Field, Input } from '@/components/ui/Input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Slider,
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/primitives';
import { queryKeys } from '@/features/queryKeys';
import { useAppFolders, useLibraryStorage } from '@/features/useLibrary';
import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { acceleratorFromEvent, formatAccelerator, volumeToPercent } from '@/lib/utils';
import { useUiStore, type SettingsTab } from '@/stores/useUiStore';
import type {
  AppSettings,
  PlaybackMode,
  ShortcutAction,
  ShortcutBinding,
  ThemePreference,
} from '@/types/domain';
import { SHORTCUT_ACTION_LABELS } from '@/types/domain';

interface SectionProps {
  settings: AppSettings;
  onPatch: (patch: Parameters<typeof ipc.updateSettings>[0]) => void;
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4 py-2">
      <div className="min-w-0">
        <p className="text-sm text-fg-default">{label}</p>
        {hint ? <p className="mt-0.5 text-xs leading-relaxed text-fg-subtle">{hint}</p> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

// --- General ----------------------------------------------------------------

function GeneralSection({ settings, onPatch }: SectionProps) {
  const pushToast = useUiStore((state) => state.pushToast);
  const general = settings.general;

  const set = (partial: Partial<typeof general>) =>
    onPatch({ general: { ...general, ...partial } });

  const autostart = useMutation({
    mutationFn: (enabled: boolean) => ipc.setAutostart(enabled),
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  return (
    <div className="divide-y divide-border-subtle">
      <Row
        label="Iniciar con el sistema"
        hint="Sound Deck arranca minimizado en la bandeja al iniciar sesion."
      >
        <Switch
          checked={general.startWithSystem}
          onCheckedChange={(checked) => autostart.mutate(checked)}
          aria-label="Iniciar con el sistema"
        />
      </Row>
      <Row
        label="Minimizar a bandeja"
        hint="Minimizar oculta la ventana en lugar de la barra de tareas."
      >
        <Switch
          checked={general.minimizeToTray}
          onCheckedChange={(minimizeToTray) => set({ minimizeToTray })}
          aria-label="Minimizar a bandeja"
        />
      </Row>
      <Row label="Cerrar a bandeja" hint="La X oculta la ventana y la aplicacion sigue corriendo.">
        <Switch
          checked={general.closeToTray}
          onCheckedChange={(closeToTray) => set({ closeToTray })}
          aria-label="Cerrar a bandeja"
        />
      </Row>
      <Row label="Mostrar notificaciones">
        <Switch
          checked={general.showNotifications}
          onCheckedChange={(showNotifications) => set({ showNotifications })}
          aria-label="Mostrar notificaciones"
        />
      </Row>
      <Row label="Abrir overlay en el monitor activo">
        <Switch
          checked={general.overlayOnActiveMonitor}
          onCheckedChange={(overlayOnActiveMonitor) => set({ overlayOnActiveMonitor })}
          aria-label="Abrir overlay en el monitor activo"
        />
      </Row>
      <Row
        label="Cerrar overlay despues de reproducir"
        hint="Recomendado: vuelve el foco al juego o programa anterior."
      >
        <Switch
          checked={general.closeOverlayAfterPlay}
          onCheckedChange={(closeOverlayAfterPlay) => set({ closeOverlayAfterPlay })}
          aria-label="Cerrar overlay despues de reproducir"
        />
      </Row>
      <Row label="Cerrar overlay al perder el foco">
        <Switch
          checked={general.closeOverlayOnBlur}
          onCheckedChange={(closeOverlayOnBlur) => set({ closeOverlayOnBlur })}
          aria-label="Cerrar overlay al perder el foco"
        />
      </Row>
      <Row label="Recordar la ultima pagina">
        <Switch
          checked={general.rememberLastPage}
          onCheckedChange={(rememberLastPage) => set({ rememberLastPage })}
          aria-label="Recordar la ultima pagina"
        />
      </Row>
      <Row label="Tema">
        <Select
          value={general.theme}
          onValueChange={(value) => set({ theme: value as ThemePreference })}
        >
          <SelectTrigger className="w-40" aria-label="Tema">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="system">Sistema</SelectItem>
            <SelectItem value="dark">Oscuro</SelectItem>
            <SelectItem value="light">Claro</SelectItem>
          </SelectContent>
        </Select>
      </Row>
      <Row
        label="Idioma"
        hint="Por ahora solo espanol. La interfaz esta preparada para agregar mas."
      >
        <Select value={general.language} onValueChange={(language) => set({ language })}>
          <SelectTrigger className="w-40" aria-label="Idioma">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="es">Espanol</SelectItem>
          </SelectContent>
        </Select>
      </Row>
    </div>
  );
}

// --- Audio ------------------------------------------------------------------

function AudioSection({ settings, onPatch }: SectionProps) {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  const audio = settings.audio;
  const [showVirtualGuide, setShowVirtualGuide] = useState(false);

  const devices = useQuery({ queryKey: queryKeys.devices, queryFn: ipc.listAudioDevices });

  const set = (partial: Partial<typeof audio>) => onPatch({ audio: { ...audio, ...partial } });

  const selectDevice = useMutation({
    mutationFn: (deviceKey: string) => {
      if (deviceKey === 'default') return ipc.useDefaultAudioDevice();
      const device = devices.data?.devices.find((candidate) => candidate.id === deviceKey);
      return ipc.selectAudioDevice(device?.id ?? null, device?.name ?? null);
    },
    onSuccess: (device) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.devices });
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      pushToast('success', `Salida: ${device.name}`);
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const testDevice = useMutation({
    mutationFn: ipc.testAudioDevice,
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const currentKey =
    devices.data?.current?.id ?? (audio.outputDeviceId ? audio.outputDeviceId : 'default');

  return (
    <div className="divide-y divide-border-subtle">
      <div className="flex flex-col gap-2 py-3">
        <Field
          label="Dispositivo de salida"
          hint="Se recuerda entre reinicios y se reconecta al arrancar."
        >
          <div className="flex gap-2">
            <Select
              value={audio.outputDeviceId ?? 'default'}
              onValueChange={(value) => selectDevice.mutate(value)}
            >
              <SelectTrigger aria-label="Dispositivo de salida">
                <SelectValue placeholder="Predeterminado del sistema" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="default">Predeterminado del sistema</SelectItem>
                {(devices.data?.devices ?? [])
                  .filter((device) => device.id !== null)
                  .map((device) => (
                    <SelectItem key={device.id} value={device.id as string}>
                      {device.name}
                      {device.isDefault ? ' (predeterminado)' : ''}
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>

            <Button
              variant="secondary"
              onClick={() => void devices.refetch()}
              disabled={devices.isFetching}
              aria-label="Actualizar dispositivos"
            >
              {devices.isFetching ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              ) : (
                <RotateCcw className="h-4 w-4" aria-hidden />
              )}
            </Button>

            <Button variant="secondary" onClick={() => testDevice.mutate()}>
              <Volume2 className="h-4 w-4" aria-hidden />
              Probar
            </Button>
          </div>
        </Field>

        {devices.data?.current && currentKey !== audio.outputDeviceId ? (
          <p className="text-xs text-warning">
            Sonando en &ldquo;{devices.data.current.name}&rdquo;.
          </p>
        ) : null}
      </div>

      <div className="py-3">
        <Field label={`Volumen general — ${volumeToPercent(audio.masterVolume)}%`}>
          <Slider
            value={[volumeToPercent(audio.masterVolume)]}
            onValueChange={([value]) => set({ masterVolume: (value ?? 0) / 100 })}
            max={100}
            step={1}
            aria-label="Volumen general"
          />
        </Field>
      </div>

      <div className="py-3">
        <Field
          label={`Volumen de previsualizacion — ${volumeToPercent(audio.previewVolume)}%`}
          hint="Se usa al escuchar audios desde la biblioteca, sin afectar la reproduccion de los botones."
        >
          <Slider
            value={[volumeToPercent(audio.previewVolume)]}
            onValueChange={([value]) => set({ previewVolume: (value ?? 0) / 100 })}
            max={100}
            step={1}
            aria-label="Volumen de previsualizacion"
          />
        </Field>
      </div>

      <Row
        label="Modo de reproduccion"
        hint="Interrumpir corta el sonido anterior; superponer permite varios a la vez."
      >
        <Select
          value={audio.playbackMode}
          onValueChange={(value) => set({ playbackMode: value as PlaybackMode })}
        >
          <SelectTrigger className="w-44" aria-label="Modo de reproduccion">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="interrupt">Interrumpir</SelectItem>
            <SelectItem value="overlap">Superponer</SelectItem>
          </SelectContent>
        </Select>
      </Row>

      <Row
        label="Reiniciar el mismo audio"
        hint="Volver a disparar un sonido que ya suena lo empieza desde el principio."
      >
        <Switch
          checked={audio.restartSameSound}
          onCheckedChange={(restartSameSound) => set({ restartSameSound })}
          aria-label="Reiniciar el mismo audio"
        />
      </Row>

      <div className="py-3">
        <Button variant="ghost" size="sm" onClick={() => setShowVirtualGuide((value) => !value)}>
          <Cable className="h-3.5 w-3.5" aria-hidden />
          {showVirtualGuide ? 'Ocultar' : 'Mostrar'} guia de dispositivo virtual
        </Button>

        {showVirtualGuide ? (
          <div className="mt-2 space-y-3 rounded-md border border-border-subtle bg-surface-2 p-3 text-xs leading-relaxed text-fg-muted">
            <p>
              Sound Deck <strong className="text-fg-default">no crea un microfono virtual</strong>.
              Solo reproduce en el dispositivo de salida que elijas.
            </p>
            <div>
              <p className="mb-1 font-medium text-fg-default">Parlantes normales</p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {`Sound Deck  ->  Parlantes / auriculares`}
              </pre>
            </div>
            <div>
              <p className="mb-1 font-medium text-fg-default">Discord y juegos</p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {`Sound Deck  ->  Salida virtual (ej. CABLE Input)
                        |
                        v
Discord: microfono = entrada virtual (CABLE Output)`}
              </pre>
              <p className="mt-1">
                Instala un dispositivo virtual de salida (VB-Cable o equivalente), elegilo aca como
                salida y configura su entrada correspondiente como microfono en Discord.
              </p>
            </div>
            <div>
              <p className="mb-1 font-medium text-fg-default">OBS</p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {`Sound Deck  ->  Salida virtual
OBS: agregar "Captura de entrada de audio" -> entrada virtual`}
              </pre>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}

// --- Atajos -----------------------------------------------------------------

function ShortcutCapture({
  binding,
  onCapture,
  disabled,
}: {
  binding: ShortcutBinding;
  onCapture: (accelerator: string) => void;
  disabled: boolean;
}) {
  const [capturing, setCapturing] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (!capturing) return;

    const handler = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === 'Escape') {
        setCapturing(false);
        setPreview(null);
        return;
      }

      const accelerator = acceleratorFromEvent(event);
      if (!accelerator) return;

      setPreview(accelerator);
      setCapturing(false);
      onCapture(accelerator);
    };

    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  }, [capturing, onCapture]);

  return (
    <Button
      variant={capturing ? 'primary' : 'secondary'}
      size="sm"
      disabled={disabled}
      onClick={() => {
        setCapturing(true);
        setPreview(null);
      }}
      className="min-w-44 justify-center font-mono"
      aria-label={`Cambiar atajo de ${SHORTCUT_ACTION_LABELS[binding.action]}`}
    >
      {capturing ? 'Presiona la combinacion...' : formatAccelerator(preview ?? binding.accelerator)}
    </Button>
  );
}

function ShortcutsSection({ settings }: SectionProps) {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);

  const update = useMutation({
    mutationFn: ({ action, accelerator }: { action: ShortcutAction; accelerator: string }) =>
      ipc.registerShortcut(action, accelerator),
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      pushToast('success', `Atajo actualizado: ${formatAccelerator(result.applied)}`);
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const reset = useMutation({
    mutationFn: ipc.resetShortcuts,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      pushToast('info', 'Atajos restablecidos a los valores predeterminados.');
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs leading-relaxed text-fg-subtle">
        Los atajos <strong className="text-fg-default">globales</strong> funcionan en todo el
        sistema. Los de <strong className="text-fg-default">overlay</strong> solo mientras el
        overlay tiene el foco. Las teclas 1 a 9 reproducen los botones dentro del overlay.
      </p>

      <div className="divide-y divide-border-subtle">
        {settings.shortcuts.bindings.map((binding) => (
          <Row
            key={binding.action}
            label={SHORTCUT_ACTION_LABELS[binding.action]}
            hint={
              binding.scope === 'global' ? 'Global — todo el sistema' : 'Solo dentro del overlay'
            }
          >
            <ShortcutCapture
              binding={binding}
              disabled={update.isPending}
              onCapture={(accelerator) => update.mutate({ action: binding.action, accelerator })}
            />
          </Row>
        ))}
      </div>

      <div>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => reset.mutate()}
          disabled={reset.isPending}
        >
          <RotateCcw className="h-3.5 w-3.5" aria-hidden />
          Restaurar atajos predeterminados
        </Button>
      </div>
    </div>
  );
}

// --- Biblioteca -------------------------------------------------------------

function LibrarySection() {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  const storage = useLibraryStorage();
  const folders = useAppFolders();

  const run = <T,>(action: () => Promise<T>, success: (value: T) => string) =>
    action()
      .then((value) => {
        pushToast('success', success(value));
        void queryClient.invalidateQueries({ queryKey: queryKeys.storage });
        void queryClient.invalidateQueries({ queryKey: ['sounds'] });
      })
      .catch((error: unknown) => pushToast('error', errorMessage(error)));

  return (
    <div className="divide-y divide-border-subtle">
      <Row label="Carpeta de sonidos" hint={folders.data?.sounds ?? 'Resolviendo...'}>
        <Button
          variant="secondary"
          size="sm"
          disabled={!folders.data}
          onClick={() => {
            if (folders.data) void openPath(folders.data.sounds);
          }}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          Abrir
        </Button>
      </Row>

      <Row
        label="Espacio utilizado"
        hint={
          storage.data
            ? `${storage.data.usedReadable} en la carpeta administrada.`
            : 'Calculando...'
        }
      >
        <span className="font-mono text-sm tabular-nums text-fg-muted">
          {storage.data?.usedReadable ?? '—'}
        </span>
      </Row>

      <Row
        label="Limpiar temporales"
        hint="Borra restos de descargas o importaciones interrumpidas."
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.cleanTempFiles, (count) =>
              count > 0 ? `${count} archivos temporales borrados.` : 'No habia temporales.',
            )
          }
        >
          Limpiar
        </Button>
      </Row>

      <Row
        label="Buscar archivos faltantes"
        hint={
          storage.data?.missingFiles
            ? `${storage.data.missingFiles} audios apuntan a archivos que ya no existen.`
            : 'Revisa si algun audio de la biblioteca perdio su archivo.'
        }
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.findMissingSounds, (missing) =>
              missing.length > 0
                ? `${missing.length} audios sin archivo.`
                : 'Todos los audios tienen su archivo.',
            )
          }
        >
          Revisar
        </Button>
      </Row>

      <Row
        label="Eliminar registros huerfanos"
        hint="Quita de la biblioteca los audios cuyo archivo ya no existe. Tambien libera sus botones."
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.removeOrphanSounds, (count) =>
              count > 0 ? `${count} registros eliminados.` : 'No habia registros huerfanos.',
            )
          }
        >
          Limpiar
        </Button>
      </Row>

      <Row
        label="Copia de seguridad"
        hint="Guarda una copia de la base de datos en la carpeta backups."
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void run(ipc.backupDatabase, () => 'Copia de seguridad creada.')}
        >
          Exportar
        </Button>
      </Row>
    </div>
  );
}

// --- Proveedores ------------------------------------------------------------

/**
 * Instrucciones por proveedor. Viven en el frontend porque son texto de
 * interfaz, no reglas de negocio: el backend no necesita saber de esto.
 */
const PROVIDER_HELP: Record<string, { steps: React.ReactNode; warning?: React.ReactNode }> = {
  freesound: {
    steps: (
      <>
        <p className="font-medium text-fg-default">Como sacar la API key</p>
        <ol className="mt-1 list-inside list-decimal space-y-1">
          <li>
            Crea una cuenta gratuita en freesound.org y entra a{' '}
            <span className="font-mono text-fg-default">freesound.org/apiv2/apply</span>.
          </li>
          <li>Completa solo el nombre y la descripcion de la aplicacion.</li>
          <li>
            El formulario tambien pide una <strong>URL de callback</strong>: es unicamente para
            OAuth2. Sound Deck usa autenticacion por token, asi que podes dejarla vacia o poner
            cualquier direccion; no se usa.
          </li>
          <li>
            Al guardar, la clave aparece al instante en la tabla de credenciales. Copiala y pegala
            aca abajo. La que necesitas es el <strong>API key</strong>, no el client secret.
          </li>
        </ol>
        <p className="mt-1.5">
          Freesound es un banco de sonidos y efectos con licencias claras. No es un sitio de memes:
          si buscas audios de ese tipo, vas a encontrar mas en un proveedor no oficial.
        </p>
      </>
    ),
  },
  myinstants: {
    steps: (
      <>
        <p className="font-medium text-fg-default">No necesita configuracion</p>
        <p className="mt-1">
          Alcanza con activarlo. Sound Deck consulta las mismas paginas publicas de busqueda que
          verias en un navegador, solo cuando escribis algo, y espaciando las consultas.
        </p>
      </>
    ),
    warning: (
      <>
        MyInstants no tiene API oficial: si el sitio cambia su estructura, este proveedor puede
        dejar de funcionar de un dia para el otro. Los audios los sube la comunidad y{' '}
        <strong>no declaran licencia</strong>, asi que revisa vos que podes hacer con cada uno antes
        de usarlo en algo publico.
      </>
    ),
  },
};

function ProvidersSection() {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  const providers = useQuery({ queryKey: queryKeys.providers, queryFn: ipc.listProviders });
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const refresh = () => void queryClient.invalidateQueries({ queryKey: queryKeys.providers });

  const toggle = useMutation({
    mutationFn: ({ providerId, enabled }: { providerId: string; enabled: boolean }) =>
      ipc.setProviderEnabled(providerId, enabled),
    onSuccess: refresh,
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const saveKey = useMutation({
    mutationFn: ({ providerId, apiKey }: { providerId: string; apiKey: string | null }) =>
      ipc.setProviderApiKey(providerId, apiKey),
    onSuccess: (_, variables) => {
      refresh();
      setDrafts((current) => ({ ...current, [variables.providerId]: '' }));
      pushToast('success', 'API key guardada.');
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const test = useMutation({
    mutationFn: (providerId: string) => ipc.testProviderConnection(providerId),
    onSuccess: () => pushToast('success', 'Conexion correcta.'),
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  return (
    <div className="flex flex-col gap-3">
      {(providers.data ?? []).map((provider) => (
        <div
          key={provider.id}
          className="flex flex-col gap-3 rounded-md border border-border-subtle bg-surface-2 p-3"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium text-fg-default">{provider.displayName}</p>
                {provider.unofficial ? (
                  <span className="rounded bg-warning/20 px-1.5 py-0.5 text-[10px] text-warning">
                    No oficial
                  </span>
                ) : null}
                {provider.ready ? (
                  <span className="rounded bg-success/20 px-1.5 py-0.5 text-[10px] text-success">
                    Listo
                  </span>
                ) : null}
              </div>
              <button
                type="button"
                onClick={() => void openUrl(provider.homepage)}
                className="mt-0.5 text-xs text-fg-subtle underline-offset-2 hover:text-accent hover:underline"
              >
                Ver terminos y condiciones del servicio
              </button>
            </div>

            <Switch
              checked={provider.enabled}
              onCheckedChange={(enabled) => toggle.mutate({ providerId: provider.id, enabled })}
              aria-label={`Activar ${provider.displayName}`}
            />
          </div>

          {PROVIDER_HELP[provider.id]?.warning ? (
            <p className="flex items-start gap-1.5 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] leading-relaxed text-warning">
              <AlertTriangle className="mt-px h-3 w-3 shrink-0" aria-hidden />
              <span>{PROVIDER_HELP[provider.id]?.warning}</span>
            </p>
          ) : null}

          {PROVIDER_HELP[provider.id]?.steps ? (
            <div className="rounded border border-border-subtle bg-surface-1 px-2.5 py-2 text-[11px] leading-relaxed text-fg-muted">
              {PROVIDER_HELP[provider.id]?.steps}
            </div>
          ) : null}

          {provider.requiresApiKey ? (
            <Field
              label="API key"
              hint={
                provider.hasApiKey
                  ? `Guardada (${provider.maskedApiKey}). Escribi una nueva para reemplazarla.`
                  : 'Necesaria para buscar. Se guarda solo en tu computadora y nunca aparece en los logs.'
              }
            >
              <div className="flex gap-2">
                <Input
                  type="password"
                  autoComplete="off"
                  placeholder={provider.hasApiKey ? '••••••••' : 'Pega tu API key'}
                  value={drafts[provider.id] ?? ''}
                  onChange={(event) =>
                    setDrafts((current) => ({ ...current, [provider.id]: event.target.value }))
                  }
                  aria-label={`API key de ${provider.displayName}`}
                />
                <Button
                  variant="secondary"
                  disabled={!(drafts[provider.id] ?? '').trim()}
                  onClick={() =>
                    saveKey.mutate({
                      providerId: provider.id,
                      apiKey: drafts[provider.id] ?? '',
                    })
                  }
                >
                  Guardar
                </Button>
                {provider.hasApiKey ? (
                  <Button
                    variant="ghost"
                    onClick={() => saveKey.mutate({ providerId: provider.id, apiKey: null })}
                  >
                    Borrar
                  </Button>
                ) : null}
              </div>
            </Field>
          ) : null}

          <div>
            <Button
              variant="secondary"
              size="sm"
              disabled={!provider.ready || test.isPending}
              onClick={() => test.mutate(provider.id)}
            >
              {test.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
              ) : (
                <Wifi className="h-3.5 w-3.5" aria-hidden />
              )}
              Probar conexion
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}

// --- Avanzado ---------------------------------------------------------------

function AdvancedSection({ settings, onPatch }: SectionProps & { version: string }) {
  const pushToast = useUiStore((state) => state.pushToast);
  const queryClient = useQueryClient();
  const folders = useAppFolders();

  const reset = useMutation({
    mutationFn: ipc.resetSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      pushToast('info', 'Configuracion restablecida.');
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  return (
    <div className="divide-y divide-border-subtle">
      <Row label="Carpeta de logs" hint={folders.data?.logs ?? 'Resolviendo...'}>
        <Button
          variant="secondary"
          size="sm"
          disabled={!folders.data}
          onClick={() => {
            if (folders.data) void openPath(folders.data.logs);
          }}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          Abrir
        </Button>
      </Row>

      <Row label="Nivel de logs" hint="Cambia el detalle registrado. Se aplica al instante.">
        <Select
          value={settings.library.logLevel}
          onValueChange={(logLevel) => onPatch({ library: { ...settings.library, logLevel } })}
        >
          <SelectTrigger className="w-36" aria-label="Nivel de logs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="error">Error</SelectItem>
            <SelectItem value="warn">Advertencia</SelectItem>
            <SelectItem value="info">Info</SelectItem>
            <SelectItem value="debug">Debug</SelectItem>
            <SelectItem value="trace">Trace</SelectItem>
          </SelectContent>
        </Select>
      </Row>

      <Row
        label="Restablecer configuracion"
        hint="Vuelve a los valores predeterminados. No borra audios ni paginas."
      >
        <Button variant="danger" size="sm" onClick={() => reset.mutate()}>
          Restablecer
        </Button>
      </Row>
    </div>
  );
}

// --- Creditos ---------------------------------------------------------------

/** Dependencia mostrada en los creditos, con su licencia. */
const CREDITS: Array<{ group: string; items: Array<[string, string, string]> }> = [
  {
    group: 'Aplicacion',
    items: [
      ['Tauri 2', 'Apache-2.0 / MIT', 'Ventanas nativas, bandeja y atajos globales'],
      ['React + TypeScript', 'MIT', 'Interfaz'],
      ['Vite', 'MIT', 'Build y desarrollo'],
      ['Tailwind CSS', 'MIT', 'Estilos'],
      ['Radix UI', 'MIT', 'Primitivas accesibles'],
      ['Lucide', 'ISC', 'Iconos'],
      ['TanStack Query / Virtual', 'MIT', 'Estado asincronico y listas virtualizadas'],
      ['Zustand', 'MIT', 'Estado de interfaz'],
    ],
  },
  {
    group: 'Audio',
    items: [
      ['rodio', 'MIT / Apache-2.0', 'Decodificacion y mezcla'],
      ['cpal', 'Apache-2.0', 'Enumeracion y apertura de dispositivos'],
      ['Symphonia', 'MPL-2.0', 'Decodificadores MP3, FLAC, Vorbis y WAV'],
    ],
  },
  {
    group: 'Datos y red',
    items: [
      ['SQLite (rusqlite)', 'Dominio publico / MIT', 'Base de datos local embebida'],
      ['reqwest + tokio', 'MIT / Apache-2.0', 'Descargas y consultas a proveedores'],
      ['scraper', 'ISC', 'Parseo HTML del proveedor no oficial'],
      ['sha2', 'MIT / Apache-2.0', 'Hash de contenido para deduplicar'],
    ],
  },
];

function CreditsSection({ version }: { version: string }) {
  const folders = useAppFolders();

  return (
    <div className="flex flex-col gap-4 text-sm">
      <div>
        <p className="text-base font-semibold text-fg-default">Sound Deck {version}</p>
        <p className="mt-1 text-xs leading-relaxed text-fg-muted">
          Soundboard de escritorio local-first. Tus audios viven en tu computadora y funcionan sin
          conexion. Codigo bajo licencia MIT.
        </p>
      </div>

      <div className="rounded-md border border-border-subtle bg-surface-2 p-3">
        <p className="text-xs font-medium text-fg-default">Sobre el contenido</p>
        <p className="mt-1 text-xs leading-relaxed text-fg-muted">
          Sound Deck no incluye ningun audio de terceros: el tono de prueba se genera en memoria.
          Los audios que descargues conservan la licencia de su origen, que queda guardada en la
          metadata de cada sonido junto con su atribucion. Revisar que podes hacer con cada uno
          antes de usarlo en algo publico queda de tu lado.
        </p>
      </div>

      {CREDITS.map(({ group, items }) => (
        <div key={group}>
          <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-fg-subtle">
            {group}
          </p>
          <div className="divide-y divide-border-subtle">
            {items.map(([name, license, purpose]) => (
              <div key={name} className="flex items-baseline gap-3 py-1.5">
                <span className="w-44 shrink-0 text-xs font-medium text-fg-default">{name}</span>
                <span className="w-40 shrink-0 font-mono text-[10px] text-fg-subtle">
                  {license}
                </span>
                <span className="min-w-0 flex-1 text-xs text-fg-muted">{purpose}</span>
              </div>
            ))}
          </div>
        </div>
      ))}

      <p className="text-xs leading-relaxed text-fg-subtle">
        El detalle completo de dependencias y licencias esta en{' '}
        <span className="font-mono text-fg-muted">THIRD-PARTY.md</span>, dentro del repositorio.
      </p>

      {folders.data ? (
        <Button
          variant="secondary"
          size="sm"
          className="self-start"
          onClick={() => void openPath(folders.data.data)}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          Abrir carpeta de datos
        </Button>
      ) : null}
    </div>
  );
}

// --- Dialogo ----------------------------------------------------------------

export interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: AppSettings;
  version: string;
}

export function SettingsDialog({ open, onOpenChange, settings, version }: SettingsDialogProps) {
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  // La pestana vive en el store para poder abrir la configuracion ya parada en
  // una seccion concreta (por ejemplo, Proveedores desde la pestana Internet).
  const tab = useUiStore((state) => state.settingsTab);
  const setTab = useUiStore((state) => state.setSettingsTab);

  const patch = useMutation({
    mutationFn: ipc.updateSettings,
    onSuccess: (updated) => {
      queryClient.setQueryData(queryKeys.settings, updated);
      void queryClient.invalidateQueries({ queryKey: queryKeys.appState });
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const onPatch = (value: Parameters<typeof ipc.updateSettings>[0]) => patch.mutate(value);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* Altura fija: el modal no cambia de tamano al pasar de una pestana a
          otra, aunque una tenga mas contenido que las demas. */}
      <DialogContent className="h-[80vh] max-w-3xl">
        <DialogHeader title="Configuracion" description={`Sound Deck ${version}`} />

        <Tabs
          value={tab}
          onValueChange={(value) => setTab(value as SettingsTab)}
          className="flex min-h-0 flex-1 flex-col"
        >
          <div className="border-b border-border-subtle px-5 py-2">
            <TabsList>
              <TabsTrigger value="general">
                <Settings2 className="h-3 w-3" aria-hidden />
                General
              </TabsTrigger>
              <TabsTrigger value="audio">
                <AudioLines className="h-3 w-3" aria-hidden />
                Audio
              </TabsTrigger>
              <TabsTrigger value="shortcuts">
                <Keyboard className="h-3 w-3" aria-hidden />
                Atajos
              </TabsTrigger>
              <TabsTrigger value="library">
                <Library className="h-3 w-3" aria-hidden />
                Biblioteca
              </TabsTrigger>
              <TabsTrigger value="providers">
                <Wifi className="h-3 w-3" aria-hidden />
                Proveedores
              </TabsTrigger>
              <TabsTrigger value="advanced">
                <Sliders className="h-3 w-3" aria-hidden />
                Avanzado
              </TabsTrigger>
              <TabsTrigger value="credits">
                <Heart className="h-3 w-3" aria-hidden />
                Creditos
              </TabsTrigger>
            </TabsList>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto px-5 py-3">
            <TabsContent value="general">
              <GeneralSection settings={settings} onPatch={onPatch} />
            </TabsContent>
            <TabsContent value="audio">
              <AudioSection settings={settings} onPatch={onPatch} />
            </TabsContent>
            <TabsContent value="shortcuts">
              <ShortcutsSection settings={settings} onPatch={onPatch} />
            </TabsContent>
            <TabsContent value="library">
              <LibrarySection />
            </TabsContent>
            <TabsContent value="providers">
              <ProvidersSection />
            </TabsContent>
            <TabsContent value="advanced">
              <AdvancedSection settings={settings} onPatch={onPatch} version={version} />
            </TabsContent>
            <TabsContent value="credits">
              <CreditsSection version={version} />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
