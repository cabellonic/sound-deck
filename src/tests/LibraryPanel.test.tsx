import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { LibraryPanel } from '@/components/library/LibraryPanel';
import { es } from '@/i18n';
import type * as IpcModule from '@/lib/ipc';

import { renderWithProviders } from './renderApp';

// La busqueda nunca resuelve: deja el panel congelado en su estado de carga,
// que es justo lo que hay que mirar.
vi.mock('@/lib/ipc', async () => {
  const actual = await vi.importActual<typeof IpcModule>('@/lib/ipc');
  return {
    ...actual,
    searchLocalSounds: vi.fn().mockReturnValue(new Promise<never>(() => {})),
    getLibraryFacets: vi.fn().mockResolvedValue({
      total: 0,
      unassigned: 0,
      categories: [],
      providers: [],
    }),
    listProviders: vi.fn().mockResolvedValue([]),
    getSettings: vi.fn().mockReturnValue(new Promise<never>(() => {})),
  };
});

function renderPanel() {
  return renderWithProviders(
    <LibraryPanel
      onAssignLocal={vi.fn()}
      onAssignRemote={vi.fn()}
      onRenameSound={vi.fn()}
      onEditSoundVolume={vi.fn()}
      onDeleteSound={vi.fn()}
      onRevealSound={vi.fn()}
      onOpenUrl={vi.fn()}
      onOpenSettings={vi.fn()}
      fileDropActive={false}
      imageDropKey={null}
    />,
  );
}

describe('LibraryPanel', () => {
  it('el indicador de carga gira', () => {
    const { container } = renderPanel();

    // Un spinner quieto se lee como "se colgo", no como "esperá".
    expect(screen.getByText(es['library.loading'])).toBeInTheDocument();
    expect(container.querySelector('.animate-spin')).not.toBeNull();
  });
});
