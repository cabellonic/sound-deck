import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import {
  AlertTriangle,
  AudioLines,
  Cable,
  ExternalLink,
  FolderOpen,
  Heart,
  Keyboard,
  Library,
  Loader2,
  Move,
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
import { LOCALE_NAMES, LOCALES, resolveLocale, shortcutKey, type PlainKey } from '@/i18n';
import { useTranslation } from '@/i18n/useTranslation';
import { useAppFolders, useLibraryStorage } from '@/features/useLibrary';
import * as ipc from '@/lib/ipc';
import { errorMessage } from '@/lib/ipc';
import { acceleratorFromEvent, formatAccelerator, volumeToPercent } from '@/lib/utils';
import { useUiStore, type SettingsTab } from '@/stores/useUiStore';
import type {
  AppSettings,
  GeneralSettings,
  PlaybackMode,
  ShortcutAction,
  ProviderStatus,
  ShortcutBinding,
  SlotModifier,
  ThemePreference,
} from '@/types/domain';
import { SLOT_MODIFIER_PREFIX, SLOT_MODIFIERS } from '@/types/domain';

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

/**
 * Tamanos del overlay, en pixeles logicos y con la proporcion de la ventana
 * original. `medium` es el tamano de fabrica, y por eso se guarda como `null`.
 */
const OVERLAY_SIZES = {
  small: { width: 420, height: 372 },
  medium: { width: 520, height: 460 },
  large: { width: 660, height: 584 },
} as const;

type OverlaySizeName = keyof typeof OVERLAY_SIZES | 'custom';

function overlaySizeName(size: GeneralSettings['overlaySize']): OverlaySizeName {
  if (!size) return 'medium';

  const match = Object.entries(OVERLAY_SIZES).find(
    ([, preset]) => preset.width === size.width && preset.height === size.height,
  );
  return (match?.[0] as OverlaySizeName | undefined) ?? 'custom';
}

function GeneralSection({ settings, onPatch }: SectionProps) {
  const { t } = useTranslation();
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
      <Row label={t('settings.general.autostart')} hint={t('settings.general.autostartHint')}>
        <Switch
          checked={general.startWithSystem}
          onCheckedChange={(checked) => autostart.mutate(checked)}
          aria-label={t('settings.general.autostart')}
        />
      </Row>
      <Row
        label={t('settings.general.minimizeToTray')}
        hint={t('settings.general.minimizeToTrayHint')}
      >
        <Switch
          checked={general.minimizeToTray}
          onCheckedChange={(minimizeToTray) => set({ minimizeToTray })}
          aria-label={t('settings.general.minimizeToTray')}
        />
      </Row>
      <Row label={t('settings.general.closeToTray')} hint={t('settings.general.closeToTrayHint')}>
        <Switch
          checked={general.closeToTray}
          onCheckedChange={(closeToTray) => set({ closeToTray })}
          aria-label={t('settings.general.closeToTray')}
        />
      </Row>
      <Row label={t('settings.general.notifications')}>
        <Switch
          checked={general.showNotifications}
          onCheckedChange={(showNotifications) => set({ showNotifications })}
          aria-label={t('settings.general.notifications')}
        />
      </Row>
      <Row
        label={t('settings.general.overlayActiveMonitor')}
        hint={
          general.overlayPosition ? t('settings.general.overlayActiveMonitorIgnored') : undefined
        }
      >
        <Switch
          checked={general.overlayOnActiveMonitor}
          disabled={general.overlayPosition !== null}
          onCheckedChange={(overlayOnActiveMonitor) => set({ overlayOnActiveMonitor })}
          aria-label={t('settings.general.overlayActiveMonitor')}
        />
      </Row>

      <Row
        label={t('settings.general.overlayPosition')}
        hint={
          general.overlayPosition
            ? t('settings.general.overlayPositionFixed', {
                x: general.overlayPosition.x,
                y: general.overlayPosition.y,
              })
            : t('settings.general.overlayPositionAuto')
        }
      >
        <div className="flex items-center gap-1.5">
          {general.overlayPosition ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                void ipc
                  .clearOverlayPlacement()
                  .catch((error: unknown) => pushToast('error', errorMessage(error)))
              }
            >
              {t('settings.general.overlayCenter')}
            </Button>
          ) : null}
          <Button
            variant="secondary"
            size="sm"
            onClick={() =>
              void ipc
                .beginOverlayPlacement()
                .catch((error: unknown) => pushToast('error', errorMessage(error)))
            }
          >
            <Move className="h-3.5 w-3.5" aria-hidden />
            {t('settings.general.overlayPick')}
          </Button>
        </div>
      </Row>

      <Row
        label={t('settings.general.overlaySize')}
        hint={t('settings.general.overlaySizeHint', {
          width: general.overlaySize?.width ?? OVERLAY_SIZES.medium.width,
        })}
      >
        <Select
          value={overlaySizeName(general.overlaySize)}
          onValueChange={(value) =>
            set({
              overlaySize: value === 'medium' ? null : OVERLAY_SIZES[value as 'small' | 'large'],
            })
          }
        >
          <SelectTrigger className="w-40" aria-label={t('settings.general.overlaySize')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="small">{t('settings.general.overlaySizeSmall')}</SelectItem>
            <SelectItem value="medium">{t('settings.general.overlaySizeMedium')}</SelectItem>
            <SelectItem value="large">{t('settings.general.overlaySizeLarge')}</SelectItem>
            {/* Solo aparece si el tamano vino de arrastrar la esquina: no es
                algo que se pueda elegir desde aca. */}
            {overlaySizeName(general.overlaySize) === 'custom' ? (
              <SelectItem value="custom" disabled>
                {t('settings.general.overlaySizeCustom')}
              </SelectItem>
            ) : null}
          </SelectContent>
        </Select>
      </Row>

      <Row
        label={t('settings.general.closeOverlayAfterPlay')}
        hint={t('settings.general.closeOverlayAfterPlayHint')}
      >
        <Switch
          checked={general.closeOverlayAfterPlay}
          onCheckedChange={(closeOverlayAfterPlay) => set({ closeOverlayAfterPlay })}
          aria-label={t('settings.general.closeOverlayAfterPlay')}
        />
      </Row>
      <Row label={t('settings.general.closeOverlayOnBlur')}>
        <Switch
          checked={general.closeOverlayOnBlur}
          onCheckedChange={(closeOverlayOnBlur) => set({ closeOverlayOnBlur })}
          aria-label={t('settings.general.closeOverlayOnBlur')}
        />
      </Row>
      <Row label={t('settings.general.rememberLastPage')}>
        <Switch
          checked={general.rememberLastPage}
          onCheckedChange={(rememberLastPage) => set({ rememberLastPage })}
          aria-label={t('settings.general.rememberLastPage')}
        />
      </Row>
      <Row label={t('settings.general.theme')}>
        <Select
          value={general.theme}
          onValueChange={(value) => set({ theme: value as ThemePreference })}
        >
          <SelectTrigger className="w-40" aria-label={t('settings.general.theme')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="system">{t('settings.general.themeSystem')}</SelectItem>
            <SelectItem value="dark">{t('settings.general.themeDark')}</SelectItem>
            <SelectItem value="light">{t('settings.general.themeLight')}</SelectItem>
          </SelectContent>
        </Select>
      </Row>
      <Row
        label={t('settings.general.language')}
        hint={
          LOCALES.length > 1
            ? t('settings.general.languageHintMany')
            : t('settings.general.languageHintOne')
        }
      >
        <Select
          value={resolveLocale(general.language)}
          onValueChange={(language) => set({ language })}
        >
          <SelectTrigger className="w-40" aria-label={t('settings.general.language')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {LOCALES.map((locale) => (
              <SelectItem key={locale} value={locale}>
                {LOCALE_NAMES[locale]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Row>
    </div>
  );
}

// --- Audio ------------------------------------------------------------------

function AudioSection({ settings, onPatch }: SectionProps) {
  const { t } = useTranslation();
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
      pushToast('success', t('settings.audio.output', { device: device.name }));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const testDevice = useMutation({
    mutationFn: ipc.testAudioDevice,
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const measure = useMutation({
    mutationFn: ipc.measureLibraryLoudness,
    onSuccess: (report) => {
      void queryClient.invalidateQueries({ queryKey: ['sounds'] });
      if (report.measured === 0 && report.failed === 0) {
        pushToast('info', t('settings.audio.measuredNone'));
        return;
      }
      const failed =
        report.failed > 0 ? t('settings.audio.measureFailed', { count: report.failed }) : '';
      pushToast('success', t('settings.audio.measured', { count: report.measured, failed }));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const currentKey =
    devices.data?.current?.id ?? (audio.outputDeviceId ? audio.outputDeviceId : 'default');

  return (
    <div className="divide-y divide-border-subtle">
      <div className="flex flex-col gap-2 py-3">
        <Field label={t('settings.audio.device')} hint={t('settings.audio.deviceHint')}>
          <div className="flex gap-2">
            <Select
              value={audio.outputDeviceId ?? 'default'}
              onValueChange={(value) => selectDevice.mutate(value)}
            >
              <SelectTrigger aria-label={t('settings.audio.device')}>
                <SelectValue placeholder={t('settings.audio.deviceDefault')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="default">{t('settings.audio.deviceDefault')}</SelectItem>
                {(devices.data?.devices ?? [])
                  .filter((device) => device.id !== null)
                  .map((device) => (
                    <SelectItem key={device.id} value={device.id as string}>
                      {device.name}
                      {device.isDefault ? t('settings.audio.deviceIsDefault') : ''}
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>

            <Button
              variant="secondary"
              onClick={() => void devices.refetch()}
              disabled={devices.isFetching}
              aria-label={t('settings.audio.refreshDevices')}
            >
              {devices.isFetching ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              ) : (
                <RotateCcw className="h-4 w-4" aria-hidden />
              )}
            </Button>

            <Button variant="secondary" onClick={() => testDevice.mutate()}>
              <Volume2 className="h-4 w-4" aria-hidden />
              {t('settings.audio.test')}
            </Button>
          </div>
        </Field>

        {devices.data?.current && currentKey !== audio.outputDeviceId ? (
          <p className="text-xs text-warning">
            {t('settings.audio.playingOn', { device: devices.data.current.name })}
          </p>
        ) : null}
      </div>

      <div className="py-3">
        <Field
          label={t('settings.audio.masterVolume', {
            percent: volumeToPercent(audio.masterVolume),
          })}
        >
          <Slider
            value={[volumeToPercent(audio.masterVolume)]}
            onValueChange={([value]) => set({ masterVolume: (value ?? 0) / 100 })}
            max={100}
            step={1}
            aria-label={t('settings.audio.masterVolumeLabel')}
          />
        </Field>
      </div>

      <div className="py-3">
        <Field
          label={t('settings.audio.previewVolume', {
            percent: volumeToPercent(audio.previewVolume),
          })}
          hint={t('settings.audio.previewVolumeHint')}
        >
          <Slider
            value={[volumeToPercent(audio.previewVolume)]}
            onValueChange={([value]) => set({ previewVolume: (value ?? 0) / 100 })}
            max={100}
            step={1}
            aria-label={t('settings.audio.previewVolumeLabel')}
          />
        </Field>
      </div>

      <Row label={t('settings.audio.playbackMode')} hint={t('settings.audio.playbackModeHint')}>
        <Select
          value={audio.playbackMode}
          onValueChange={(value) => set({ playbackMode: value as PlaybackMode })}
        >
          <SelectTrigger className="w-44" aria-label={t('settings.audio.playbackMode')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="interrupt">{t('settings.audio.interrupt')}</SelectItem>
            <SelectItem value="overlap">{t('settings.audio.overlap')}</SelectItem>
          </SelectContent>
        </Select>
      </Row>

      <Row label={t('settings.audio.restartSame')} hint={t('settings.audio.restartSameHint')}>
        <Switch
          checked={audio.restartSameSound}
          onCheckedChange={(restartSameSound) => set({ restartSameSound })}
          aria-label={t('settings.audio.restartSame')}
        />
      </Row>

      <Row label={t('settings.audio.normalize')} hint={t('settings.audio.normalizeHint')}>
        <Switch
          checked={audio.normalizeVolume}
          onCheckedChange={(normalizeVolume) => set({ normalizeVolume })}
          aria-label={t('settings.audio.normalize')}
        />
      </Row>

      {audio.normalizeVolume ? (
        <Row label={t('settings.audio.measure')} hint={t('settings.audio.measureHint')}>
          <Button
            variant="secondary"
            size="sm"
            disabled={measure.isPending}
            onClick={() => measure.mutate()}
          >
            {measure.isPending ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
            ) : (
              <AudioLines className="h-3.5 w-3.5" aria-hidden />
            )}
            {measure.isPending ? t('settings.audio.measuring') : t('settings.audio.measureAction')}
          </Button>
        </Row>
      ) : null}

      <div className="py-3">
        <Button variant="ghost" size="sm" onClick={() => setShowVirtualGuide((value) => !value)}>
          <Cable className="h-3.5 w-3.5" aria-hidden />
          {showVirtualGuide ? t('settings.audio.hideGuide') : t('settings.audio.showGuide')}
        </Button>

        {showVirtualGuide ? (
          <div className="mt-2 space-y-3 rounded-md border border-border-subtle bg-surface-2 p-3 text-xs leading-relaxed text-fg-muted">
            <p>{t('settings.audio.guideIntro')}</p>
            <div>
              <p className="mb-1 font-medium text-fg-default">
                {t('settings.audio.guideSpeakers')}
              </p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {t('settings.audio.diagramSpeakers')}
              </pre>
            </div>
            <div>
              <p className="mb-1 font-medium text-fg-default">{t('settings.audio.guideDiscord')}</p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {t('settings.audio.diagramDiscord')}
              </pre>
              <p className="mt-1">{t('settings.audio.guideDiscordHint')}</p>
            </div>
            <div>
              <p className="mb-1 font-medium text-fg-default">{t('settings.audio.guideObs')}</p>
              <pre className="overflow-x-auto rounded bg-surface-0 p-2 font-mono text-[11px]">
                {t('settings.audio.diagramObs')}
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
  const { t } = useTranslation();
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
      aria-label={t('settings.shortcuts.change', { action: t(shortcutKey(binding.action)) })}
    >
      {capturing
        ? t('settings.shortcuts.pressCombo')
        : formatAccelerator(preview ?? binding.accelerator, t)}
    </Button>
  );
}

function ShortcutsSection({ settings, onPatch }: SectionProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);

  const update = useMutation({
    mutationFn: ({ action, accelerator }: { action: ShortcutAction; accelerator: string }) =>
      ipc.registerShortcut(action, accelerator),
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      pushToast(
        'success',
        t('settings.shortcuts.updated', { accelerator: formatAccelerator(result.applied, t) }),
      );
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const reset = useMutation({
    mutationFn: ipc.resetShortcuts,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      pushToast('info', t('settings.shortcuts.resetDone'));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const shortcuts = settings.shortcuts;
  const set = (partial: Partial<typeof shortcuts>) =>
    onPatch({ shortcuts: { ...shortcuts, ...partial } });

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs leading-relaxed text-fg-subtle">{t('settings.shortcuts.intro')}</p>

      <div className="divide-y divide-border-subtle">
        {settings.shortcuts.bindings.map((binding) => (
          <Row
            key={binding.action}
            label={t(shortcutKey(binding.action))}
            hint={
              binding.scope === 'global'
                ? t('settings.shortcuts.scopeGlobal')
                : t('settings.shortcuts.scopeOverlay')
            }
          >
            <ShortcutCapture
              binding={binding}
              disabled={update.isPending}
              onCapture={(accelerator) => update.mutate({ action: binding.action, accelerator })}
            />
          </Row>
        ))}

        <Row
          label={t('settings.shortcuts.globalSlots')}
          hint={t('settings.shortcuts.globalSlotsHint')}
        >
          <Switch
            checked={shortcuts.globalSlotPlayback}
            onCheckedChange={(globalSlotPlayback) => set({ globalSlotPlayback })}
            aria-label={t('settings.shortcuts.globalSlots')}
          />
        </Row>

        {shortcuts.globalSlotPlayback ? (
          <Row
            label={t('settings.shortcuts.slotModifier')}
            hint={t('settings.shortcuts.slotModifierHint', {
              first: formatAccelerator(`${SLOT_MODIFIER_PREFIX[shortcuts.slotModifier]}+1`, t),
              last: formatAccelerator(`${SLOT_MODIFIER_PREFIX[shortcuts.slotModifier]}+9`, t),
            })}
          >
            <Select
              value={shortcuts.slotModifier}
              onValueChange={(value) => set({ slotModifier: value as SlotModifier })}
            >
              <SelectTrigger
                className="w-44"
                aria-label={t('settings.shortcuts.slotModifierLabel')}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {SLOT_MODIFIERS.map((modifier) => (
                  <SelectItem key={modifier} value={modifier}>
                    {formatAccelerator(SLOT_MODIFIER_PREFIX[modifier], t)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Row>
        ) : null}
      </div>

      <div>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => reset.mutate()}
          disabled={reset.isPending}
        >
          <RotateCcw className="h-3.5 w-3.5" aria-hidden />
          {t('settings.shortcuts.reset')}
        </Button>
      </div>
    </div>
  );
}

// --- Biblioteca -------------------------------------------------------------

function LibrarySection() {
  const { t } = useTranslation();
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

  /**
   * Restaurar reemplaza toda la biblioteca, asi que se confirma antes. El
   * backend valida la copia y guarda la base actual antes de tocar nada; si
   * todo sale bien la aplicacion se reinicia y este codigo no sigue.
   */
  const pickBackup = async () => {
    try {
      const selected = await open({
        multiple: false,
        title: t('settings.library.restoreTitle'),
        defaultPath: folders.data?.data,
        filters: [{ name: t('settings.library.restoreFilter'), extensions: ['sqlite'] }],
      });
      if (typeof selected !== 'string') return;

      const confirmed = await confirm(t('settings.library.restoreConfirm'), {
        title: t('settings.library.restoreTitle'),
        kind: 'warning',
        okLabel: t('settings.library.restoreOk'),
      });
      if (!confirmed) return;

      await ipc.restoreDatabase(selected);
    } catch (error) {
      pushToast('error', errorMessage(error));
    }
  };

  return (
    <div className="divide-y divide-border-subtle">
      <Row
        label={t('settings.library.soundsFolder')}
        hint={folders.data?.sounds ?? t('settings.resolvingPath')}
      >
        <Button
          variant="secondary"
          size="sm"
          disabled={!folders.data}
          onClick={() => {
            if (folders.data) void openPath(folders.data.sounds);
          }}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          {t('common.open')}
        </Button>
      </Row>

      <Row
        label={t('settings.library.usedSpace')}
        hint={
          storage.data
            ? t('settings.library.usedSpaceHint', { size: storage.data.usedReadable })
            : t('settings.library.calculating')
        }
      >
        <span className="font-mono text-sm tabular-nums text-fg-muted">
          {storage.data?.usedReadable ?? '—'}
        </span>
      </Row>

      <Row label={t('settings.library.cleanTemp')} hint={t('settings.library.cleanTempHint')}>
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.cleanTempFiles, (count) =>
              count > 0
                ? t('settings.library.cleanedTemp', { count })
                : t('settings.library.noTemp'),
            )
          }
        >
          {t('settings.library.clean')}
        </Button>
      </Row>

      <Row
        label={t('settings.library.findMissing')}
        hint={
          storage.data?.missingFiles
            ? t('settings.library.missingCount', { count: storage.data.missingFiles })
            : t('settings.library.findMissingHint')
        }
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.findMissingSounds, (missing) =>
              missing.length > 0
                ? t('settings.library.missingFound', { count: missing.length })
                : t('settings.library.noneMissing'),
            )
          }
        >
          {t('settings.library.check')}
        </Button>
      </Row>

      <Row
        label={t('settings.library.removeOrphans')}
        hint={t('settings.library.removeOrphansHint')}
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            void run(ipc.removeOrphanSounds, (count) =>
              count > 0
                ? t('settings.library.orphansRemoved', { count })
                : t('settings.library.noOrphans'),
            )
          }
        >
          {t('settings.library.clean')}
        </Button>
      </Row>

      <Row label={t('settings.library.backup')} hint={t('settings.library.backupHint')}>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void run(ipc.backupDatabase, () => t('settings.library.backupDone'))}
        >
          {t('common.export')}
        </Button>
      </Row>

      <Row label={t('settings.library.restore')} hint={t('settings.library.restoreHint')}>
        <Button variant="secondary" size="sm" onClick={() => void pickBackup()}>
          {t('common.import')}
        </Button>
      </Row>
    </div>
  );
}

// --- Proveedores ------------------------------------------------------------

/**
 * Instrucciones por proveedor. Viven en el frontend porque son texto de
 * interfaz, no reglas de negocio: el backend no necesita saber de esto.
 *
 * Son claves y no JSX para que se puedan traducir enteras. Meter `<strong>` en
 * el medio de una frase obliga a partirla en pedazos, y una frase partida no
 * sobrevive a un idioma con otro orden de palabras.
 */
const PROVIDER_HELP: Record<
  string,
  { title: PlainKey; steps?: PlainKey[]; note?: PlainKey; warning?: PlainKey }
> = {
  freesound: {
    title: 'help.freesound.title',
    steps: [
      'help.freesound.step1',
      'help.freesound.step2',
      'help.freesound.step3',
      'help.freesound.step4',
    ],
    note: 'help.freesound.note',
  },
  myinstants: {
    title: 'help.myinstants.title',
    note: 'help.myinstants.note',
    warning: 'help.myinstants.warning',
  },
};

/** Bloque de ayuda de un proveedor, ya traducido. */
function ProviderHelp({ providerId }: { providerId: string }) {
  const { t } = useTranslation();
  const help = PROVIDER_HELP[providerId];
  if (!help) return null;

  return (
    <div className="rounded border border-border-subtle bg-surface-1 px-2.5 py-2 text-[11px] leading-relaxed text-fg-muted">
      <p className="font-medium text-fg-default">{t(help.title)}</p>
      {help.steps ? (
        <ol className="mt-1 list-inside list-decimal space-y-1">
          {help.steps.map((step) => (
            <li key={step}>{t(step)}</li>
          ))}
        </ol>
      ) : null}
      {help.note ? <p className="mt-1.5">{t(help.note)}</p> : null}
    </div>
  );
}

function ProvidersSection() {
  const { t } = useTranslation();
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
      pushToast('success', t('settings.providers.apiKeySavedToast'));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const test = useMutation({
    mutationFn: (providerId: string) => ipc.testProviderConnection(providerId),
    onSuccess: () => pushToast('success', t('settings.providers.connectionOk')),
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
                    {t('settings.providers.unofficial')}
                  </span>
                ) : null}
                {provider.ready ? (
                  <span className="rounded bg-success/20 px-1.5 py-0.5 text-[10px] text-success">
                    {t('settings.providers.ready')}
                  </span>
                ) : null}
              </div>
              <button
                type="button"
                onClick={() => void openUrl(provider.homepage)}
                className="mt-0.5 text-xs text-fg-subtle underline-offset-2 hover:text-accent hover:underline"
              >
                {t('settings.providers.terms')}
              </button>
            </div>

            <Switch
              checked={provider.enabled}
              onCheckedChange={(enabled) => toggle.mutate({ providerId: provider.id, enabled })}
              aria-label={t('settings.providers.enable', { provider: provider.displayName })}
            />
          </div>

          {PROVIDER_HELP[provider.id]?.warning ? (
            <p className="flex items-start gap-1.5 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] leading-relaxed text-warning">
              <AlertTriangle className="mt-px h-3 w-3 shrink-0" aria-hidden />
              <span>{t(PROVIDER_HELP[provider.id]!.warning!)}</span>
            </p>
          ) : null}

          <ProviderHelp providerId={provider.id} />

          {provider.requiresApiKey ? (
            <Field
              label={t('settings.providers.apiKey')}
              hint={
                provider.hasApiKey
                  ? t('settings.providers.apiKeySaved', { masked: provider.maskedApiKey ?? '' })
                  : t('settings.providers.apiKeyHint')
              }
            >
              <div className="flex gap-2">
                <Input
                  type="password"
                  autoComplete="off"
                  placeholder={
                    provider.hasApiKey ? '••••••••' : t('settings.providers.apiKeyPlaceholder')
                  }
                  value={drafts[provider.id] ?? ''}
                  onChange={(event) =>
                    setDrafts((current) => ({ ...current, [provider.id]: event.target.value }))
                  }
                  aria-label={t('settings.providers.apiKeyLabel', {
                    provider: provider.displayName,
                  })}
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
                  {t('common.save')}
                </Button>
                {provider.hasApiKey ? (
                  <Button
                    variant="ghost"
                    onClick={() => saveKey.mutate({ providerId: provider.id, apiKey: null })}
                  >
                    {t('common.delete')}
                  </Button>
                ) : null}
              </div>
            </Field>
          ) : null}

          {provider.supportsOauth ? <ProviderAccount provider={provider} /> : null}

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
              {t('settings.providers.testConnection')}
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * Conexion de la cuenta del proveedor por OAuth2.
 *
 * Sin cuenta conectada Freesound solo entrega la preview MP3; con ella se baja
 * el archivo original. El codigo se pega a mano en vez de escuchar en un puerto
 * local: es un paso mas para el usuario, pero nada queda escuchando en su
 * maquina y no hay puerto que pueda estar ocupado.
 */
function ProviderAccount({ provider }: { provider: ProviderStatus }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const pushToast = useUiStore((state) => state.pushToast);
  const [clientId, setClientId] = useState('');
  const [code, setCode] = useState('');

  const refresh = () => void queryClient.invalidateQueries({ queryKey: queryKeys.providers });

  const saveClientId = useMutation({
    mutationFn: (value: string | null) => ipc.setProviderClientId(provider.id, value),
    onSuccess: () => {
      refresh();
      setClientId('');
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const authorize = useMutation({
    mutationFn: () => ipc.beginProviderAuthorization(provider.id),
    onSuccess: (request) => void openUrl(request.url),
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const complete = useMutation({
    mutationFn: (value: string) => ipc.completeProviderAuthorization(provider.id, value),
    onSuccess: () => {
      refresh();
      setCode('');
      pushToast('success', t('settings.providers.connected'));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  const disconnect = useMutation({
    mutationFn: () => ipc.disconnectProviderAccount(provider.id),
    onSuccess: () => {
      refresh();
      pushToast('info', t('settings.providers.disconnected'));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  if (provider.accountConnected) {
    return (
      <div className="flex items-center justify-between gap-3 rounded border border-success/40 bg-success/10 px-2.5 py-2">
        <p className="text-[11px] leading-relaxed text-fg-muted">
          <strong className="text-success">{t('settings.providers.accountConnected')}</strong>{' '}
          {t('settings.providers.accountConnectedHint')}
        </p>
        <Button
          variant="ghost"
          size="sm"
          disabled={disconnect.isPending}
          onClick={() => disconnect.mutate()}
        >
          {t('settings.providers.disconnect')}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 rounded border border-border-subtle bg-surface-1 px-2.5 py-2">
      <p className="text-[11px] leading-relaxed text-fg-muted">
        {t('settings.providers.accountIntro')}
      </p>

      <Field
        label={t('settings.providers.clientId')}
        hint={
          provider.hasClientId
            ? t('settings.providers.clientIdSaved')
            : t('settings.providers.clientIdHint')
        }
      >
        <div className="flex gap-2">
          <Input
            autoComplete="off"
            placeholder={
              provider.hasClientId ? '••••••••' : t('settings.providers.clientIdPlaceholder')
            }
            value={clientId}
            onChange={(event) => setClientId(event.target.value)}
            aria-label={t('settings.providers.clientIdLabel', {
              provider: provider.displayName,
            })}
          />
          <Button
            variant="secondary"
            disabled={!clientId.trim()}
            onClick={() => saveClientId.mutate(clientId)}
          >
            {t('common.save')}
          </Button>
        </div>
      </Field>

      {provider.hasClientId ? (
        <>
          <div>
            <Button
              variant="secondary"
              size="sm"
              disabled={!provider.hasApiKey || authorize.isPending}
              onClick={() => authorize.mutate()}
            >
              <ExternalLink className="h-3.5 w-3.5" aria-hidden />
              {t('settings.providers.authorize')}
            </Button>
          </div>

          <Field label={t('settings.providers.code')} hint={t('settings.providers.codeHint')}>
            <div className="flex gap-2">
              <Input
                autoComplete="off"
                placeholder={t('settings.providers.codePlaceholder')}
                value={code}
                onChange={(event) => setCode(event.target.value)}
                aria-label={t('settings.providers.codeLabel')}
              />
              <Button
                variant="secondary"
                disabled={!code.trim() || complete.isPending}
                onClick={() => complete.mutate(code)}
              >
                {complete.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
                ) : null}
                {t('settings.providers.connect')}
              </Button>
            </div>
          </Field>
        </>
      ) : null}
    </div>
  );
}

// --- Avanzado ---------------------------------------------------------------

function AdvancedSection({ settings, onPatch }: SectionProps & { version: string }) {
  const { t } = useTranslation();
  const pushToast = useUiStore((state) => state.pushToast);
  const queryClient = useQueryClient();
  const folders = useAppFolders();

  const reset = useMutation({
    mutationFn: ipc.resetSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      pushToast('info', t('settings.advanced.resetDone'));
    },
    onError: (error) => pushToast('error', errorMessage(error)),
  });

  return (
    <div className="divide-y divide-border-subtle">
      <Row
        label={t('settings.advanced.logsFolder')}
        hint={folders.data?.logs ?? t('settings.resolvingPath')}
      >
        <Button
          variant="secondary"
          size="sm"
          disabled={!folders.data}
          onClick={() => {
            if (folders.data) void openPath(folders.data.logs);
          }}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          {t('common.open')}
        </Button>
      </Row>

      <Row label={t('settings.advanced.logLevel')} hint={t('settings.advanced.logLevelHint')}>
        <Select
          value={settings.library.logLevel}
          onValueChange={(logLevel) => onPatch({ library: { ...settings.library, logLevel } })}
        >
          <SelectTrigger className="w-36" aria-label={t('settings.advanced.logLevel')}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="error">{t('settings.advanced.logError')}</SelectItem>
            <SelectItem value="warn">{t('settings.advanced.logWarn')}</SelectItem>
            <SelectItem value="info">{t('settings.advanced.logInfo')}</SelectItem>
            <SelectItem value="debug">{t('settings.advanced.logDebug')}</SelectItem>
            <SelectItem value="trace">{t('settings.advanced.logTrace')}</SelectItem>
          </SelectContent>
        </Select>
      </Row>

      <Row label={t('settings.advanced.reset')} hint={t('settings.advanced.resetHint')}>
        <Button variant="danger" size="sm" onClick={() => reset.mutate()}>
          {t('settings.advanced.resetAction')}
        </Button>
      </Row>
    </div>
  );
}

// --- Creditos ---------------------------------------------------------------

/** Dependencia mostrada en los creditos, con su licencia. */
const CREDITS: Array<{ group: PlainKey; items: Array<[string, string, PlainKey]> }> = [
  {
    group: 'settings.credits.groupApp',
    items: [
      ['Tauri 2', 'Apache-2.0 / MIT', 'credits.tauri'],
      ['React + TypeScript', 'MIT', 'credits.react'],
      ['Vite', 'MIT', 'credits.vite'],
      ['Tailwind CSS', 'MIT', 'credits.tailwind'],
      ['Radix UI', 'MIT', 'credits.radix'],
      ['Lucide', 'ISC', 'credits.lucide'],
      ['TanStack Query / Virtual', 'MIT', 'credits.tanstack'],
      ['Zustand', 'MIT', 'credits.zustand'],
    ],
  },
  {
    group: 'settings.credits.groupAudio',
    items: [
      ['rodio', 'MIT / Apache-2.0', 'credits.rodio'],
      ['cpal', 'Apache-2.0', 'credits.cpal'],
      ['Symphonia', 'MPL-2.0', 'credits.symphonia'],
      ['ebur128', 'MIT', 'credits.ebur128'],
    ],
  },
  {
    group: 'settings.credits.groupData',
    items: [
      ['SQLite (rusqlite)', 'Dominio publico / MIT', 'credits.sqlite'],
      ['reqwest + tokio', 'MIT / Apache-2.0', 'credits.reqwest'],
      ['scraper', 'ISC', 'credits.scraper'],
      ['sha2', 'MIT / Apache-2.0', 'credits.sha2'],
    ],
  },
];

function CreditsSection({ version }: { version: string }) {
  const { t } = useTranslation();
  const folders = useAppFolders();

  return (
    <div className="flex flex-col gap-4 text-sm">
      <div>
        <p className="text-base font-semibold text-fg-default">Sound Deck {version}</p>
        <p className="mt-1 text-xs leading-relaxed text-fg-muted">
          {t('settings.credits.tagline')}
        </p>
      </div>

      <div className="rounded-md border border-border-subtle bg-surface-2 p-3">
        <p className="text-xs font-medium text-fg-default">{t('settings.credits.contentTitle')}</p>
        <p className="mt-1 text-xs leading-relaxed text-fg-muted">
          {t('settings.credits.content')}
        </p>
      </div>

      {CREDITS.map(({ group, items }) => (
        <div key={group}>
          <p className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-fg-subtle">
            {t(group)}
          </p>
          <div className="divide-y divide-border-subtle">
            {items.map(([name, license, purpose]) => (
              <div key={name} className="flex items-baseline gap-3 py-1.5">
                <span className="w-44 shrink-0 text-xs font-medium text-fg-default">{name}</span>
                <span className="w-40 shrink-0 font-mono text-[10px] text-fg-subtle">
                  {license}
                </span>
                <span className="min-w-0 flex-1 text-xs text-fg-muted">{t(purpose)}</span>
              </div>
            ))}
          </div>
        </div>
      ))}

      <p className="text-xs leading-relaxed text-fg-subtle">
        {t('settings.credits.authorBefore')}
        <a href="https://github.com/cabellonic" className="text-accent underline">
          cabellonic
        </a>
        {t('settings.credits.authorAfter')}
      </p>

      {folders.data ? (
        <Button
          variant="secondary"
          size="sm"
          className="self-start"
          onClick={() => void openPath(folders.data.data)}
        >
          <FolderOpen className="h-3.5 w-3.5" aria-hidden />
          {t('settings.credits.openDataFolder')}
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
  const { t } = useTranslation();
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
        <DialogHeader title={t('settings.title')} description={`Sound Deck ${version}`} />

        <Tabs
          value={tab}
          onValueChange={(value) => setTab(value as SettingsTab)}
          className="flex min-h-0 flex-1 flex-col"
        >
          <div className="border-b border-border-subtle px-5 py-2">
            <TabsList>
              <TabsTrigger value="general">
                <Settings2 className="h-3 w-3" aria-hidden />
                {t('settings.tab.general')}
              </TabsTrigger>
              <TabsTrigger value="audio">
                <AudioLines className="h-3 w-3" aria-hidden />
                {t('settings.tab.audio')}
              </TabsTrigger>
              <TabsTrigger value="shortcuts">
                <Keyboard className="h-3 w-3" aria-hidden />
                {t('settings.tab.shortcuts')}
              </TabsTrigger>
              <TabsTrigger value="library">
                <Library className="h-3 w-3" aria-hidden />
                {t('settings.tab.library')}
              </TabsTrigger>
              <TabsTrigger value="providers">
                <Wifi className="h-3 w-3" aria-hidden />
                {t('settings.tab.providers')}
              </TabsTrigger>
              <TabsTrigger value="advanced">
                <Sliders className="h-3 w-3" aria-hidden />
                {t('settings.tab.advanced')}
              </TabsTrigger>
              <TabsTrigger value="credits">
                <Heart className="h-3 w-3" aria-hidden />
                {t('settings.tab.credits')}
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
