import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { isClickSuppressed, useDragSource, useDragStore, useDropTarget } from '@/features/dnd';
import type { DragPayload } from '@/lib/drag';

afterEach(() => {
  act(() => useDragStore.getState().finish());
});

function Source({ payload, label }: { payload: DragPayload | null; label: string }) {
  const { onPointerDown } = useDragSource(() => payload, label);
  return (
    <button type="button" data-testid="source" onPointerDown={onPointerDown}>
      origen
    </button>
  );
}

function Target({ id, onDrop }: { id: string; onDrop: (p: DragPayload) => void }) {
  const { dropProps, isOver } = useDropTarget(id, onDrop);
  return (
    <div {...dropProps} data-testid="target">
      {isOver ? 'encima' : 'libre'}
    </div>
  );
}

/** Emite un `pointermove` global con las coordenadas dadas. */
function movePointer(x: number, y: number) {
  act(() => {
    window.dispatchEvent(new PointerEvent('pointermove', { clientX: x, clientY: y }));
  });
}

function releasePointer(x: number, y: number) {
  act(() => {
    window.dispatchEvent(new PointerEvent('pointerup', { clientX: x, clientY: y }));
  });
}

describe('drag and drop por punteros', () => {
  it('no empieza a arrastrar por un movimiento menor al umbral', () => {
    render(<Source payload={{ kind: 'local-sound', soundId: 'a' }} label="A" />);

    const source = screen.getByTestId('source');
    act(() => {
      source.dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 100, bubbles: true }),
      );
    });

    movePointer(102, 101); // 2px: por debajo del umbral
    expect(useDragStore.getState().payload).toBeNull();

    releasePointer(102, 101);
  });

  it('arrastra tras superar el umbral y suelta sobre el destino valido', () => {
    const onDrop = vi.fn();
    render(
      <>
        <Source payload={{ kind: 'local-sound', soundId: 'a' }} label="Bruh" />
        <Target id="slot:1" onDrop={onDrop} />
      </>,
    );

    // El destino esta en un punto conocido de la pantalla.
    const target = screen.getByTestId('target');
    target.getBoundingClientRect = () =>
      ({ left: 200, top: 200, right: 260, bottom: 240, width: 60, height: 40 }) as DOMRect;
    vi.spyOn(document, 'elementFromPoint').mockImplementation((x, y) =>
      x >= 200 && x <= 260 && y >= 200 && y <= 240 ? target : document.body,
    );

    const source = screen.getByTestId('source');
    act(() => {
      source.dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 100, bubbles: true }),
      );
    });

    movePointer(120, 120); // supera el umbral: empieza el arrastre
    expect(useDragStore.getState().payload).toEqual({ kind: 'local-sound', soundId: 'a' });
    expect(useDragStore.getState().label).toBe('Bruh');

    movePointer(230, 220); // sobre el destino
    expect(screen.getByTestId('target')).toHaveTextContent('encima');

    releasePointer(230, 220);
    expect(onDrop).toHaveBeenCalledWith({ kind: 'local-sound', soundId: 'a' });
    // El arrastre termino: el fantasma desaparece.
    expect(useDragStore.getState().payload).toBeNull();
  });

  it('suprime el clic que sigue a un arrastre', () => {
    render(<Source payload={{ kind: 'local-sound', soundId: 'a' }} label="A" />);
    vi.spyOn(document, 'elementFromPoint').mockReturnValue(document.body);

    const source = screen.getByTestId('source');
    act(() => {
      source.dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 100, bubbles: true }),
      );
    });
    movePointer(140, 140);
    releasePointer(140, 140);

    // Justo despues de soltar, un clic debe ignorarse (fue el fin del arrastre).
    expect(isClickSuppressed()).toBe(true);
  });

  it('no arrastra con el boton secundario', () => {
    render(<Source payload={{ kind: 'local-sound', soundId: 'a' }} label="A" />);

    const source = screen.getByTestId('source');
    act(() => {
      source.dispatchEvent(
        new PointerEvent('pointerdown', { button: 2, clientX: 100, clientY: 100, bubbles: true }),
      );
    });
    movePointer(140, 140);

    expect(useDragStore.getState().payload).toBeNull();
  });

  it('no arrastra cuando el origen no tiene payload', () => {
    render(<Source payload={null} label="vacio" />);

    const source = screen.getByTestId('source');
    act(() => {
      source.dispatchEvent(
        new PointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 100, bubbles: true }),
      );
    });
    movePointer(140, 140);

    expect(useDragStore.getState().payload).toBeNull();
  });
});
