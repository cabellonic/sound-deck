import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { columnsForWidth } from '@/components/library/columns';
import { soundAt } from '@/features/useFileDrop';

/** jsdom no hace hit-testing: le decimos que elemento hay bajo el cursor. */
function pointingAt(element: Element | null) {
  vi.spyOn(document, 'elementFromPoint').mockReturnValue(element);
}

describe('destino de una imagen soltada sobre la biblioteca', () => {
  it('resuelve una fila de un audio guardado', () => {
    render(
      <div data-sound-drop="local" data-sound-id="sound-1" data-sound-name="Bruh">
        <span data-testid="hijo">Bruh</span>
      </div>,
    );

    // Se suelta sobre un hijo de la fila, que es lo que pasa de verdad.
    pointingAt(screen.getByTestId('hijo'));

    expect(soundAt(10, 10)).toEqual({
      kind: 'local',
      key: 'local:sound-1',
      soundId: 'sound-1',
      name: 'Bruh',
    });
  });

  it('resuelve un resultado online sin descargar', () => {
    render(
      <div
        data-testid="fila"
        data-sound-drop="remote"
        data-provider-id="freesound"
        data-remote-id="42"
        data-sound-name="Risa"
      />,
    );
    pointingAt(screen.getByTestId('fila'));

    expect(soundAt(10, 10)).toEqual({
      kind: 'remote',
      key: 'remote:freesound:42',
      providerId: 'freesound',
      remoteId: '42',
      name: 'Risa',
      savedSoundId: null,
    });
  });

  it('trae el id local cuando el resultado ya se descargo', () => {
    render(
      <div
        data-testid="fila"
        data-sound-drop="remote"
        data-provider-id="freesound"
        data-remote-id="42"
        data-sound-name="Risa"
        data-saved-sound-id="sound-9"
      />,
    );
    pointingAt(screen.getByTestId('fila'));

    // Con el audio ya en la biblioteca no hace falta preguntar nada: la imagen
    // se le asigna directo.
    expect(soundAt(10, 10)).toMatchObject({ savedSoundId: 'sound-9' });
  });

  it('no resuelve nada fuera de una fila', () => {
    render(<div data-testid="cualquiera">sin atributos</div>);
    pointingAt(screen.getByTestId('cualquiera'));

    expect(soundAt(10, 10)).toBeNull();
  });

  it('ignora una fila a la que le falta lo que la identifica', () => {
    render(<div data-testid="fila" data-sound-drop="remote" data-provider-id="freesound" />);
    pointingAt(screen.getByTestId('fila'));

    expect(soundAt(10, 10)).toBeNull();
  });
});

describe('columnas de la biblioteca', () => {
  it('crece con el ancho disponible', () => {
    expect(columnsForWidth(400)).toBe(1);
    expect(columnsForWidth(700)).toBe(2);
    // Pantalla completa en 1920: la biblioteca queda con ~1550px.
    expect(columnsForWidth(1550)).toBe(3);
  });

  it('no pasa de tres, aunque haya lugar', () => {
    expect(columnsForWidth(3840)).toBe(3);
  });
});
