import { describe, expect, it } from 'vitest';

import {
  acceleratorFromEvent,
  formatAccelerator,
  formatBytes,
  formatDuration,
  isEditableTarget,
  percentToVolume,
  slotFromKey,
  volumeToPercent,
} from '@/lib/utils';

describe('formatDuration', () => {
  it('formatea minutos y segundos', () => {
    expect(formatDuration(1234)).toBe('0:01');
    expect(formatDuration(65_000)).toBe('1:05');
    expect(formatDuration(600_000)).toBe('10:00');
  });

  it('devuelve null cuando la duracion es desconocida', () => {
    expect(formatDuration(null)).toBeNull();
    expect(formatDuration(undefined)).toBeNull();
    expect(formatDuration(Number.NaN)).toBeNull();
    expect(formatDuration(-5)).toBeNull();
  });
});

describe('formatBytes', () => {
  it('usa la misma escala que el backend', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
  });

  it('tolera valores invalidos', () => {
    expect(formatBytes(null)).toBe('—');
    expect(formatBytes(-1)).toBe('—');
  });
});

describe('volumen', () => {
  it('convierte en ambos sentidos y recorta el rango', () => {
    expect(volumeToPercent(0.35)).toBe(35);
    expect(volumeToPercent(2)).toBe(100);
    expect(volumeToPercent(-1)).toBe(0);
    expect(percentToVolume(20)).toBeCloseTo(0.2);
    expect(percentToVolume(500)).toBe(1);
  });
});

describe('slotFromKey', () => {
  it('reconoce las teclas numericas y el teclado numerico', () => {
    expect(slotFromKey('Digit1', '1')).toBe(1);
    expect(slotFromKey('Numpad9', '9')).toBe(9);
  });

  it('ignora el cero y las teclas que no son numeros', () => {
    expect(slotFromKey('Digit0', '0')).toBeNull();
    expect(slotFromKey('KeyA', 'a')).toBeNull();
    expect(slotFromKey('Escape', 'Escape')).toBeNull();
  });

  it('cae al valor de `key` en distribuciones raras', () => {
    expect(slotFromKey('Unidentified', '5')).toBe(5);
  });
});

describe('isEditableTarget', () => {
  it('detecta campos de texto', () => {
    const input = document.createElement('input');
    input.type = 'text';
    expect(isEditableTarget(input)).toBe(true);

    const textarea = document.createElement('textarea');
    expect(isEditableTarget(textarea)).toBe(true);
  });

  it('no considera editables a los controles no textuales', () => {
    const range = document.createElement('input');
    range.type = 'range';
    expect(isEditableTarget(range)).toBe(false);

    const button = document.createElement('button');
    expect(isEditableTarget(button)).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});

describe('aceleradores', () => {
  it('construye el acelerador desde un evento', () => {
    const event = new KeyboardEvent('keydown', {
      code: 'Space',
      key: ' ',
      ctrlKey: true,
      altKey: true,
    });
    expect(acceleratorFromEvent(event)).toBe('Ctrl+Alt+Space');
  });

  it('ignora los eventos que solo tienen modificadores', () => {
    const event = new KeyboardEvent('keydown', {
      code: 'ControlLeft',
      key: 'Control',
      ctrlKey: true,
    });
    expect(acceleratorFromEvent(event)).toBeNull();
  });

  it('formatea para mostrar', () => {
    expect(formatAccelerator('Ctrl+Alt+Space')).toBe('Ctrl + Alt + Espacio');
    expect(formatAccelerator('PageUp')).toBe('Re Pag');
  });
});
