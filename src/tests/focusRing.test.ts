import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { initFocusRing, type StopFocusRing } from '@/lib/focusRing';

let stop: StopFocusRing | undefined;

function start() {
  stop = initFocusRing();
  return userEvent.setup();
}

afterEach(() => {
  stop?.();
  stop = undefined;
  delete document.documentElement.dataset.nav;
});

describe('initFocusRing', () => {
  it('arranca en modo mouse', () => {
    start();

    expect(document.documentElement.dataset.nav).toBe('pointer');
  });

  it('pasa a modo teclado al navegar con Tab o flechas', async () => {
    const user = start();

    await user.keyboard('{Tab}');
    expect(document.documentElement.dataset.nav).toBe('keyboard');

    await user.pointer({ target: document.body, keys: '[MouseLeft]' });
    await user.keyboard('{ArrowDown}');
    expect(document.documentElement.dataset.nav).toBe('keyboard');
  });

  it('los atajos no encienden el anillo', async () => {
    const user = start();

    // El caso que rompia: reproducir con las teclas 1-9 dejaba el anillo en lo
    // ultimo que se hubiera tocado con el mouse.
    await user.keyboard('1');
    await user.keyboard('{PageDown}');

    expect(document.documentElement.dataset.nav).toBe('pointer');
  });

  it('el mouse vuelve a apagar el anillo', async () => {
    const user = start();

    await user.keyboard('{Tab}');
    await user.pointer({ target: document.body, keys: '[MouseLeft]' });

    expect(document.documentElement.dataset.nav).toBe('pointer');
  });
});
