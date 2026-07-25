import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

import type { PlainKey } from '@/i18n';

/** Combina clases resolviendo conflictos de Tailwind. */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

/** Duracion legible: `1:05`, `0:03`. `null` cuando se desconoce (§39). */
export function formatDuration(durationMs: number | null | undefined): string | null {
  if (durationMs === null || durationMs === undefined || !Number.isFinite(durationMs)) {
    return null;
  }
  if (durationMs < 0) return null;

  const totalSeconds = Math.round(durationMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

/** Tamano legible con la misma escala que usa el backend. */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) {
    return '—';
  }

  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[unit]}`;
}

/** Porcentaje entero a partir de un volumen 0..1. */
export function volumeToPercent(volume: number): number {
  if (!Number.isFinite(volume)) return 0;
  return Math.round(Math.min(Math.max(volume, 0), 1) * 100);
}

export function percentToVolume(percent: number): number {
  if (!Number.isFinite(percent)) return 0;
  return Math.min(Math.max(percent / 100, 0), 1);
}

/**
 * Si el foco esta en un campo editable.
 *
 * Se usa para no capturar los numeros 1-9 mientras el usuario escribe (§28).
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;

  const tag = target.tagName.toLowerCase();
  if (tag === 'textarea' || tag === 'select') return true;
  if (tag === 'input') {
    const type = (target as HTMLInputElement).type;
    // Los controles no textuales (botones, sliders) no capturan texto.
    return !['button', 'checkbox', 'radio', 'range', 'submit', 'reset'].includes(type);
  }
  return false;
}

/** Convierte una tecla `Digit1`..`Digit9` al numero de slot. */
export function slotFromKey(code: string, key: string): number | null {
  const match = /^(?:Digit|Numpad)([1-9])$/.exec(code);
  if (match?.[1]) return Number(match[1]);

  // Fallback para distribuciones donde `code` no llega como se espera.
  if (/^[1-9]$/.test(key)) return Number(key);
  return null;
}

/**
 * Descompone una fecha en la clave de traduccion que le corresponde.
 *
 * La funcion no traduce: devuelve que decir y con que numero, para que el
 * catalogo se quede con el texto y esto se quede con las cuentas.
 */
export type RelativeDate =
  | { kind: 'never' }
  | { kind: 'invalid' }
  | { kind: 'now' }
  | { kind: 'minutes' | 'hours' | 'days'; value: number }
  | { kind: 'absolute'; value: string };

export function relativeDate(iso: string | null, locale: string): RelativeDate {
  if (!iso) return { kind: 'never' };

  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return { kind: 'invalid' };

  const seconds = Math.round((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return { kind: 'now' };
  if (seconds < 3600) return { kind: 'minutes', value: Math.floor(seconds / 60) };
  if (seconds < 86400) return { kind: 'hours', value: Math.floor(seconds / 3600) };
  if (seconds < 604800) return { kind: 'days', value: Math.floor(seconds / 86400) };
  return { kind: 'absolute', value: date.toLocaleDateString(locale) };
}

/**
 * Nombres de tecla que cambian con el idioma.
 *
 * Los modificadores no estan: `Ctrl`, `Alt`, `Shift` y `Win` se escriben igual
 * en todos los idiomas que nos importan, y traducirlos solo daria lugar a que
 * alguien invente una variante.
 */
const KEY_LABELS: Record<string, PlainKey> = {
  space: 'key.space',
  pageup: 'key.pageUp',
  pagedown: 'key.pageDown',
  home: 'key.home',
  end: 'key.end',
};

/**
 * Muestra un acelerador con nombres de tecla legibles.
 *
 * Recibe el traductor porque los nombres de tecla son texto de interfaz: en
 * ingles la tecla `Home` no se llama "Inicio".
 */
export function formatAccelerator(accelerator: string, t: (key: PlainKey) => string): string {
  return accelerator
    .split('+')
    .map((part) => {
      const lower = part.toLowerCase();
      switch (lower) {
        case 'ctrl':
          return 'Ctrl';
        case 'alt':
          return 'Alt';
        case 'shift':
          return 'Shift';
        case 'super':
          return 'Win';
        default: {
          const label = KEY_LABELS[lower];
          return label ? t(label) : part;
        }
      }
    })
    .join(' + ');
}

/**
 * Construye un acelerador a partir de un evento de teclado.
 * Devuelve `null` mientras solo hay modificadores presionados.
 */
export function acceleratorFromEvent(event: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');

  const modifierKeys = ['Control', 'Alt', 'Shift', 'Meta', 'OS'];
  if (modifierKeys.includes(event.key)) return null;

  const key = normalizeKeyName(event.code, event.key);
  if (!key) return null;

  parts.push(key);
  return parts.join('+');
}

function normalizeKeyName(code: string, key: string): string | null {
  const digit = /^(?:Digit|Numpad)(\d)$/.exec(code);
  if (digit?.[1]) return digit[1];

  const letter = /^Key([A-Z])$/.exec(code);
  if (letter?.[1]) return letter[1];

  const fKey = /^(F\d{1,2})$/.exec(code);
  if (fKey?.[1]) return fKey[1];

  switch (code) {
    case 'Space':
      return 'Space';
    case 'Escape':
      return 'Escape';
    case 'Enter':
    case 'NumpadEnter':
      return 'Enter';
    case 'PageUp':
      return 'PageUp';
    case 'PageDown':
      return 'PageDown';
    case 'Home':
      return 'Home';
    case 'End':
      return 'End';
    case 'Insert':
      return 'Insert';
    case 'Delete':
      return 'Delete';
    case 'ArrowUp':
      return 'ArrowUp';
    case 'ArrowDown':
      return 'ArrowDown';
    case 'ArrowLeft':
      return 'ArrowLeft';
    case 'ArrowRight':
      return 'ArrowRight';
    default:
      return key.length === 1 ? key.toUpperCase() : null;
  }
}
