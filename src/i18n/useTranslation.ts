/**
 * Acceso a las traducciones desde React.
 *
 * El idioma vive en la configuracion, que ya viaja por eventos: cuando cambia,
 * `settings-changed` actualiza la query y todo lo que use este hook se
 * re-renderiza solo. No hace falta un contexto propio.
 */
import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { queryKeys } from '@/features/queryKeys';
import * as ipc from '@/lib/ipc';

import {
  DEFAULT_LOCALE,
  plural,
  resolveLocale,
  translate,
  type Locale,
  type Params,
  type PlainKey,
  type TranslationKey,
} from './index';

export interface Translator {
  /** Traduce una clave. Las que llevan `{parametros}` los exigen. */
  t: {
    (key: PlainKey): string;
    <K extends TranslationKey>(key: K, params: Params<K>): string;
  };
  /** Elige entre la forma singular y la plural segun la cantidad. */
  tp: <K extends TranslationKey>(count: number, one: K, many: K, params?: Params<K>) => string;
  locale: Locale;
}

export function useTranslation(): Translator {
  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: ipc.getSettings });
  const locale = resolveLocale(settings.data?.general.language);
  return useTranslator(locale);
}

/**
 * Variante para ventanas que ya tienen la configuracion en la mano, como el
 * overlay, que la lee por su cuenta y no monta el cliente de queries.
 */
export function useTranslator(locale: Locale = DEFAULT_LOCALE): Translator {
  const t = useCallback(
    (key: TranslationKey, params?: Record<string, string | number>) =>
      translate(locale, key, params as never),
    [locale],
  );

  const tp = useCallback(
    <K extends TranslationKey>(count: number, one: K, many: K, params?: Params<K>) =>
      plural(locale, count, one, many, params),
    [locale],
  );

  return useMemo(() => ({ t: t as Translator['t'], tp, locale }), [t, tp, locale]);
}
