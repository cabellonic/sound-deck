import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { en, es, translate } from '@/i18n';
import * as ipc from '@/lib/ipc';
import type * as IpcModule from '@/lib/ipc';

import type * as factories from './factories';
import { makeSettings } from './factories';
import { renderWithProviders } from './renderApp';
import { useUiStore } from '@/stores/useUiStore';

const settings = makeSettings();

// La pantalla consulta dispositivos, proveedores y carpetas al montarse. Nada
// de eso existe fuera de Tauri, asi que se devuelve lo minimo para renderizar.
vi.mock('@/lib/ipc', async () => {
  const actual = await vi.importActual<typeof IpcModule>('@/lib/ipc');
  const { makeSettings } = await vi.importActual<typeof factories>('./factories');
  return {
    ...actual,
    // `useTranslation` la consulta para saber el idioma.
    getSettings: vi.fn().mockResolvedValue(makeSettings()),
    listAudioDevices: vi.fn().mockResolvedValue({ devices: [], current: null }),
    listProviders: vi.fn().mockResolvedValue([]),
    getAppFolders: vi.fn().mockResolvedValue({ sounds: 'C:/sounds', logs: 'C:/logs', data: 'C:/' }),
    getLibraryStorage: vi.fn().mockResolvedValue({
      soundsDir: 'C:/sounds',
      usedBytes: 0,
      usedReadable: '0 B',
      missingFiles: 0,
    }),
  };
});

function renderSettings(value = settings) {
  return renderWithProviders(
    <SettingsDialog open onOpenChange={vi.fn()} settings={value} version="0.1.0" />,
  );
}

// La pestana abierta vive en el store de Zustand, que es de modulo y sobrevive
// al `cleanup` de Testing Library: sin esto, un test hereda la pestana del anterior.
beforeEach(() => {
  useUiStore.setState({ settingsTab: 'general' });
});

describe('SettingsDialog', () => {
  beforeEach(() => {
    renderSettings();
  });

  it('saca del catalogo el titulo y las pestanas', () => {
    expect(screen.getByText(es['settings.title'])).toBeInTheDocument();

    for (const tab of [
      'settings.tab.general',
      'settings.tab.audio',
      'settings.tab.shortcuts',
      'settings.tab.library',
      'settings.tab.providers',
      'settings.tab.advanced',
      'settings.tab.credits',
    ] as const) {
      expect(screen.getByRole('tab', { name: es[tab] })).toBeInTheDocument();
    }
  });

  it('traduce las opciones de la pestana general', () => {
    expect(
      screen.getByRole('switch', { name: es['settings.general.autostart'] }),
    ).toBeInTheDocument();
    expect(screen.getByText(es['settings.general.autostartHint'])).toBeInTheDocument();
    expect(
      screen.getByRole('switch', { name: es['settings.general.minimizeToTray'] }),
    ).toBeInTheDocument();
    expect(screen.getByText(es['settings.general.overlayPositionAuto'])).toBeInTheDocument();
  });

  it('muestra el tamano del overlay sin uno guardado como el de fabrica', () => {
    expect(
      screen.getByRole('combobox', { name: es['settings.general.overlaySize'] }),
    ).toHaveTextContent(es['settings.general.overlaySizeMedium']);
    expect(
      screen.getByText(translate('es', 'settings.general.overlaySizeHint', { width: 520 })),
    ).toBeInTheDocument();
  });

  it('traduce tambien las pestanas que no estan a la vista al abrir', async () => {
    // Es donde mas facil se escapa un texto sin traducir: no se ve hasta que
    // alguien entra a la pestana.
    await userEvent.click(screen.getByRole('tab', { name: es['settings.tab.audio'] }));
    expect(screen.getByText(es['settings.audio.device'])).toBeInTheDocument();
    expect(screen.getByText(es['settings.audio.playbackMode'])).toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: es['settings.tab.shortcuts'] }));
    expect(screen.getByText(es['settings.shortcuts.intro'])).toBeInTheDocument();
    expect(screen.getByText(es['shortcut.toggle_overlay'])).toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: es['settings.tab.library'] }));
    expect(screen.getByText(es['settings.library.soundsFolder'])).toBeInTheDocument();
    expect(screen.getByText(es['settings.library.restore'])).toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: es['settings.tab.advanced'] }));
    expect(screen.getByText(es['settings.advanced.logLevel'])).toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: es['settings.tab.credits'] }));
    expect(screen.getByText(es['settings.credits.tagline'])).toBeInTheDocument();
    expect(screen.getByText(es['credits.react'])).toBeInTheDocument();
  });
});

describe('SettingsDialog en ingles', () => {
  it('rinde la interfaz en el idioma guardado', async () => {
    const ingles = makeSettings({ general: { ...settings.general, language: 'en' } });
    vi.mocked(ipc.getSettings).mockResolvedValue(ingles);
    renderSettings(ingles);

    // El idioma llega por la consulta de configuracion, no por la prop: hay que
    // esperar a que resuelva y a que el dialogo se vuelva a pintar.
    expect(
      await screen.findByText(en['settings.general.autostart'], undefined, { timeout: 5000 }),
    ).toBeInTheDocument();
    expect(screen.queryByText(es['settings.general.autostart'])).not.toBeInTheDocument();

    // Los nombres de tecla tambien: es lo que se escapaba antes de llevarlos al
    // catalogo, porque salian de una funcion y no de un componente.
    await userEvent.click(screen.getByRole('tab', { name: en['settings.tab.shortcuts'] }));
    expect(screen.getByText(en['shortcut.toggle_overlay'])).toBeInTheDocument();
    // El boton lleva `aria-label`, asi que el acelerador es su contenido.
    expect(screen.getByText('Alt + Home')).toBeInTheDocument();
    expect(screen.queryByText('Alt + Inicio')).not.toBeInTheDocument();
  });
});
