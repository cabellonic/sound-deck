/**
 * Traduccion de la interfaz (§43).
 *
 * Es una implementacion propia y no `i18next` por una razon concreta: aca las
 * claves salen del catalogo en espanol, asi que TypeScript falla al compilar si
 * alguien escribe una clave que no existe o se olvida un parametro. Una
 * libreria generica acepta cualquier string y el error aparece en pantalla.
 *
 * Agregar un idioma es agregar un archivo con las mismas claves y sumarlo a
 * `CATALOGS`. Los componentes no se tocan.
 */
import { es } from './es';
import { en } from './en';

/** El catalogo en espanol define las claves; los demas deben cumplirlas. */
export type Catalog = { [K in keyof typeof es]: string };
export type TranslationKey = keyof typeof es;

export const LOCALES = ['es', 'en'] as const;
export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = 'es';

/** Nombre de cada idioma en su propio idioma, para el selector de Ajustes. */
export const LOCALE_NAMES: Record<Locale, string> = {
  es: 'Espanol',
  en: 'English',
};

const CATALOGS: Record<Locale, Catalog> = { es, en };

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value);
}

/**
 * Idioma a usar a partir de lo que hay guardado.
 *
 * Un idioma que ya no existe (porque se quito, o porque la configuracion viene
 * de una version mas nueva) cae al predeterminado en vez de dejar la interfaz
 * sin textos.
 */
export function resolveLocale(preferred: string | undefined): Locale {
  return preferred && isLocale(preferred) ? preferred : DEFAULT_LOCALE;
}

/** Parametros de una clave: `{nombre}` en el texto se vuelve `{ nombre: ... }`. */
type Placeholders<T extends string> = T extends `${string}{${infer Name}}${infer Rest}`
  ? { [K in Name | keyof Placeholders<Rest>]: string | number }
  : Record<never, never>;

export type Params<K extends TranslationKey> = Placeholders<(typeof es)[K]>;

/** Claves que no llevan ningun parametro, para poder llamarlas con un argumento. */
export type PlainKey = {
  [K in TranslationKey]: keyof Params<K> extends never ? K : never;
}[TranslationKey];

/**
 * Reemplaza los `{parametro}` del texto.
 *
 * Un parametro que falta se deja tal cual en vez de imprimir `undefined`: es
 * mas facil de ver en una captura de pantalla que un hueco vacio.
 */
export function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export function translate<K extends TranslationKey>(
  locale: Locale,
  key: K,
  params?: Params<K>,
): string {
  const catalog = CATALOGS[locale] ?? CATALOGS[DEFAULT_LOCALE];
  // El fallback al espanol evita que una traduccion incompleta deje la interfaz
  // con claves crudas a la vista.
  const template: string = catalog[key] ?? CATALOGS[DEFAULT_LOCALE][key] ?? key;
  return interpolate(template, params as Record<string, string | number> | undefined);
}

/**
 * Plural simple: elige entre dos claves segun la cantidad.
 *
 * Alcanza para espanol e ingles. Un idioma con mas formas plurales va a
 * necesitar cambiar esto, y ese es el momento de traer una libreria.
 */
export function plural<K extends TranslationKey>(
  locale: Locale,
  count: number,
  one: K,
  many: K,
  params?: Params<K>,
): string {
  return translate(locale, count === 1 ? one : many, params);
}

/** Clave de catalogo de una categoria normalizada. Ninguna lleva parametros. */
export function categoryKey(category: string): PlainKey {
  return `category.${category}` as PlainKey;
}

/** Clave de catalogo de una accion con atajo. Ninguna lleva parametros. */
export function shortcutKey(action: string): PlainKey {
  return `shortcut.${action}` as PlainKey;
}

export { en, es };
