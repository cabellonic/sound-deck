import { describe, expect, it } from 'vitest';

import {
  categoryKey,
  DEFAULT_LOCALE,
  en,
  es,
  interpolate,
  isLocale,
  plural,
  resolveLocale,
  shortcutKey,
  translate,
  type TranslationKey,
} from '@/i18n';
import { NORMALIZED_CATEGORIES, type ShortcutAction } from '@/types/domain';

const SHORTCUT_ACTIONS: readonly ShortcutAction[] = [
  'toggle_overlay',
  'stop_all',
  'prev_page',
  'next_page',
];

describe('interpolate', () => {
  it('reemplaza los parametros del texto', () => {
    expect(interpolate('En {page} · boton {slot}', { page: 'Memes', slot: 3 })).toBe(
      'En Memes · boton 3',
    );
  });

  it('deja el hueco a la vista si falta un parametro', () => {
    // Un `{name}` en pantalla se ve en cualquier captura; un `undefined` pasa
    // desapercibido hasta que alguien lo reporta.
    expect(interpolate('Hola {name}', {})).toBe('Hola {name}');
  });

  it('no toca un texto sin parametros', () => {
    expect(interpolate('Cancelar')).toBe('Cancelar');
    expect(interpolate('Cancelar', { sobra: 'x' })).toBe('Cancelar');
  });
});

describe('resolveLocale', () => {
  it('acepta los idiomas que existen', () => {
    expect(resolveLocale('es')).toBe('es');
    expect(isLocale('es')).toBe(true);
  });

  it('cae al predeterminado con un idioma desconocido', () => {
    // Pasa al volver de una version mas nueva, o si se quita un idioma: la
    // interfaz tiene que seguir teniendo textos.
    expect(resolveLocale('klingon')).toBe(DEFAULT_LOCALE);
    expect(resolveLocale(undefined)).toBe(DEFAULT_LOCALE);
    expect(isLocale('klingon')).toBe(false);
  });
});

describe('translate', () => {
  it('devuelve el texto del catalogo', () => {
    expect(translate('es', 'common.cancel')).toBe('Cancelar');
  });

  it('completa los parametros', () => {
    expect(translate('es', 'soundboard.newPage', { max: 9 })).toBe('Nueva pagina (maximo 9)');
  });

  it('elige la forma plural segun la cantidad', () => {
    expect(
      plural('es', 1, 'sound.assignedCount.one', 'sound.assignedCount.many', { count: 1 }),
    ).toBe('En 1 boton');
    expect(
      plural('es', 4, 'sound.assignedCount.one', 'sound.assignedCount.many', { count: 4 }),
    ).toBe('En 4 botones');
  });
});

/** Nombres de los `{parametros}` que usa un texto. */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]!).sort();
}

describe('paridad entre catalogos', () => {
  it('los dos idiomas tienen exactamente las mismas claves', () => {
    // TypeScript ya exige que no falte ninguna, pero una clave de mas en el
    // idioma traducido se ignora en silencio y nadie se entera.
    expect(Object.keys(en).sort()).toEqual(Object.keys(es).sort());
  });

  it('cada traduccion conserva los parametros del original', () => {
    // Lo mas facil de romper al traducir: perder un `{name}` o escribirlo mal.
    // El tipo `Params` sale del catalogo en espanol, asi que el compilador no
    // ve el error; en pantalla aparece la llave cruda o un hueco.
    const rotas = (Object.keys(es) as TranslationKey[])
      .filter((key) => placeholders(es[key]).join() !== placeholders(en[key]).join())
      .map((key) => `${key}: es=[${placeholders(es[key])}] en=[${placeholders(en[key])}]`);

    expect(rotas).toEqual([]);
  });
});

describe('catalogo', () => {
  it('no tiene textos vacios', () => {
    const vacios = Object.entries(es).filter(([, value]) => value.trim().length === 0);
    expect(vacios).toEqual([]);
  });

  it('tiene un texto para cada accion con atajo', () => {
    // Misma trampa que con las categorias: `shortcutKey` arma la clave a mano.
    for (const action of SHORTCUT_ACTIONS) {
      const key = shortcutKey(action);
      expect(es, `falta el texto del atajo ${action}`).toHaveProperty(key);
      expect(translate('es', key)).not.toBe(key);
    }
  });

  it('tiene un texto para cada categoria del backend', () => {
    // `categoryKey` construye la clave a mano, asi que TypeScript no puede
    // avisar de una que falte: se veria el nombre crudo de la clave en pantalla.
    for (const category of NORMALIZED_CATEGORIES) {
      const key = categoryKey(category);
      expect(es, `falta el texto de la categoria ${category}`).toHaveProperty(key);
      expect(translate('es', key)).not.toBe(key);
    }
  });
});
