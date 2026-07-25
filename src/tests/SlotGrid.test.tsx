import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { SlotGrid } from '@/components/soundboard/SlotGrid';

import { makePage, makeSlot, makeSound } from './factories';
import { renderWithProviders } from './renderApp';

function renderGrid(overrides: Partial<Parameters<typeof SlotGrid>[0]> = {}) {
  const props = {
    page: makePage(),
    playingSoundIds: [],
    slotDownloads: {},
    onPlay: vi.fn(),
    onDropPayload: vi.fn(),
    onClear: vi.fn(),
    onEditLabel: vi.fn(),
    onEditVolume: vi.fn(),
    onPickImage: vi.fn(),
    onClearImage: vi.fn(),
    onReveal: vi.fn(),
    onShowDetails: vi.fn(),
    ...overrides,
  };

  const { container } = renderWithProviders(<SlotGrid {...props} />);

  return { ...props, container };
}

describe('SlotGrid', () => {
  it('renderiza nueve botones numerados', () => {
    renderGrid();

    const buttons = screen.getAllByRole('button');
    expect(buttons).toHaveLength(9);
    for (let n = 1; n <= 9; n += 1) {
      expect(
        screen.getByRole('button', { name: new RegExp(`^Boton ${n}\\.`) }),
      ).toBeInTheDocument();
    }
  });

  it('muestra "Vacio" en los slots sin asignar', () => {
    renderGrid();
    expect(screen.getAllByText('Vacio')).toHaveLength(9);
  });

  it('muestra el nombre y la duracion del audio asignado', () => {
    const page = makePage();
    page.slots[0] = makeSlot({
      slotNumber: 1,
      sound: makeSound({ name: 'Risa malvada', durationMs: 65_000 }),
    });
    renderGrid({ page });

    expect(screen.getByText('Risa malvada')).toBeInTheDocument();
    expect(screen.getByText('1:05')).toBeInTheDocument();
  });

  it('prioriza la etiqueta personalizada sobre el nombre del audio', () => {
    const page = makePage();
    page.slots[0] = makeSlot({
      slotNumber: 1,
      sound: makeSound({ name: 'Nombre largo del archivo' }),
      customLabel: 'Corto',
    });
    renderGrid({ page });

    expect(screen.getByText('Corto')).toBeInTheDocument();
    expect(screen.queryByText('Nombre largo del archivo')).not.toBeInTheDocument();
  });

  it('muestra la imagen del audio asignado', () => {
    const page = makePage();
    page.slots[0] = makeSlot({
      slotNumber: 1,
      sound: makeSound({ name: 'Con imagen', imagePath: 'C:\\datos\\images\\abc.png' }),
    });
    const { container } = renderGrid({ page });

    const image = container.querySelector('img');
    expect(image).not.toBeNull();
    // La ruta se sirve por el protocolo `asset:`, nunca como ruta cruda.
    expect(image?.getAttribute('src')).toContain('asset');
    expect(image?.getAttribute('src')).not.toBe('C:\\datos\\images\\abc.png');
    // Es decorativa: el nombre del boton ya lo dice el texto y el aria-label.
    expect(image).toHaveAttribute('alt', '');
    expect(screen.getByText('Con imagen')).toBeInTheDocument();
  });

  it('deja el boton sin imagen si el audio no tiene', () => {
    const page = makePage();
    page.slots[0] = makeSlot({ slotNumber: 1, sound: makeSound({ name: 'Sin imagen' }) });
    const { container } = renderGrid({ page });

    expect(container.querySelector('img')).toBeNull();
    expect(screen.getByText('Sin imagen')).toBeInTheDocument();
  });

  it('avisa cuando el archivo del audio ya no existe', () => {
    const page = makePage();
    page.slots[2] = makeSlot({
      slotNumber: 3,
      sound: makeSound({ name: 'Roto', fileAvailable: false }),
    });
    renderGrid({ page });

    expect(screen.getByText('Archivo faltante')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /Boton 3\..*Archivo no disponible/ }),
    ).toBeInTheDocument();
  });

  it('reproduce al hacer clic', async () => {
    const user = userEvent.setup();
    const props = renderGrid();

    await user.click(screen.getByRole('button', { name: /^Boton 4\./ }));
    expect(props.onPlay).toHaveBeenCalledWith(4);
  });

  it('reproduce con Enter y con Espacio', async () => {
    const user = userEvent.setup();
    const props = renderGrid();

    const button = screen.getByRole('button', { name: /^Boton 2\./ });
    button.focus();

    await user.keyboard('{Enter}');
    await user.keyboard(' ');

    expect(props.onPlay).toHaveBeenCalledTimes(2);
    expect(props.onPlay).toHaveBeenCalledWith(2);
  });

  it('mueve el foco con las flechas', async () => {
    const user = userEvent.setup();
    renderGrid();

    screen.getByRole('button', { name: /^Boton 5\./ }).focus();

    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('button', { name: /^Boton 6\./ })).toHaveFocus();

    await user.keyboard('{ArrowDown}');
    expect(screen.getByRole('button', { name: /^Boton 9\./ })).toHaveFocus();

    await user.keyboard('{ArrowUp}');
    expect(screen.getByRole('button', { name: /^Boton 6\./ })).toHaveFocus();

    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('button', { name: /^Boton 5\./ })).toHaveFocus();
  });

  it('no salta de fila en los bordes horizontales', async () => {
    const user = userEvent.setup();
    renderGrid();

    screen.getByRole('button', { name: /^Boton 3\./ }).focus();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('button', { name: /^Boton 3\./ })).toHaveFocus();

    screen.getByRole('button', { name: /^Boton 4\./ }).focus();
    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('button', { name: /^Boton 4\./ })).toHaveFocus();
  });

  it('marca el slot que esta sonando', () => {
    const page = makePage();
    page.slots[0] = makeSlot({ slotNumber: 1, sound: makeSound({ id: 'sonando' }) });
    renderGrid({ page, playingSoundIds: ['sonando'] });

    expect(screen.getByRole('button', { name: /^Boton 1\..*Reproduciendo/ })).toBeInTheDocument();
  });

  it('deshabilita el boton mientras se descarga y expone el progreso', () => {
    renderGrid({ slotDownloads: { 7: 0.42 } });

    expect(screen.getByRole('button', { name: /^Boton 7\..*Descargando/ })).toBeDisabled();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42');
  });
});
