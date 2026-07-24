import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { useSlotKeys, type SlotKeysOptions } from '@/features/useSlotKeys';

function Harness(options: Omit<SlotKeysOptions, 'enabled'> & { enabled?: boolean }) {
  useSlotKeys({ enabled: true, ...options });
  return (
    <div>
      <input aria-label="Buscar" />
      <button type="button">Otro control</button>
    </div>
  );
}

describe('useSlotKeys', () => {
  it('dispara el slot correspondiente a la tecla', async () => {
    const user = userEvent.setup();
    const onSlot = vi.fn();
    render(<Harness onSlot={onSlot} />);

    await user.keyboard('1');
    await user.keyboard('9');

    expect(onSlot).toHaveBeenNthCalledWith(1, 1);
    expect(onSlot).toHaveBeenNthCalledWith(2, 9);
  });

  it('no dispara mientras se escribe en un input', async () => {
    const user = userEvent.setup();
    const onSlot = vi.fn();
    render(<Harness onSlot={onSlot} />);

    await user.click(screen.getByLabelText('Buscar'));
    await user.keyboard('123');

    expect(onSlot).not.toHaveBeenCalled();
    expect(screen.getByLabelText('Buscar')).toHaveValue('123');
  });

  it('ignora combinaciones con modificadores', async () => {
    const user = userEvent.setup();
    const onSlot = vi.fn();
    render(<Harness onSlot={onSlot} />);

    await user.keyboard('{Control>}1{/Control}');
    await user.keyboard('{Alt>}2{/Alt}');

    expect(onSlot).not.toHaveBeenCalled();
  });

  it('cambia de pagina con Re Pag y Av Pag', async () => {
    const user = userEvent.setup();
    const onPrevPage = vi.fn();
    const onNextPage = vi.fn();
    render(<Harness onSlot={vi.fn()} onPrevPage={onPrevPage} onNextPage={onNextPage} />);

    await user.keyboard('{PageUp}');
    await user.keyboard('{PageDown}');

    expect(onPrevPage).toHaveBeenCalledTimes(1);
    expect(onNextPage).toHaveBeenCalledTimes(1);
  });

  it('Escape llega incluso desde un input', async () => {
    const user = userEvent.setup();
    const onEscape = vi.fn();
    render(<Harness onSlot={vi.fn()} onEscape={onEscape} />);

    await user.click(screen.getByLabelText('Buscar'));
    await user.keyboard('{Escape}');

    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it('no hace nada cuando esta deshabilitado', async () => {
    const user = userEvent.setup();
    const onSlot = vi.fn();
    render(<Harness enabled={false} onSlot={onSlot} />);

    await user.keyboard('5');
    expect(onSlot).not.toHaveBeenCalled();
  });

  it('ignora la repeticion por tecla mantenida salvo que se configure', () => {
    const onSlot = vi.fn();
    const { unmount } = render(<Harness onSlot={onSlot} />);

    window.dispatchEvent(
      new KeyboardEvent('keydown', { code: 'Digit3', key: '3', repeat: true, bubbles: true }),
    );
    expect(onSlot).not.toHaveBeenCalled();
    unmount();

    render(<Harness onSlot={onSlot} allowRepeat />);
    window.dispatchEvent(
      new KeyboardEvent('keydown', { code: 'Digit3', key: '3', repeat: true, bubbles: true }),
    );
    expect(onSlot).toHaveBeenCalledWith(3);
  });
});
