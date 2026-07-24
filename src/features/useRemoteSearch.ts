/**
 * Busqueda online (§13).
 *
 * TanStack Query se encarga de la cancelacion y del orden: cada texto es una
 * clave distinta, asi que una respuesta vieja nunca pisa a una nueva. El
 * debounce de 300 ms esta dentro del rango pedido (250-400 ms).
 */
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';

import * as ipc from '@/lib/ipc';
import type { ProviderStatus } from '@/types/domain';

import { queryKeys } from './queryKeys';
import { useDebouncedValue } from './useDebouncedValue';

export const REMOTE_SEARCH_DEBOUNCE_MS = 300;
const MIN_QUERY_LENGTH = 2;

export function useProviders() {
  return useQuery({ queryKey: queryKeys.providers, queryFn: ipc.listProviders });
}

/** Si hay al menos un proveedor listo para consultarse. */
export function hasReadyProvider(providers: ProviderStatus[] | undefined): boolean {
  return (providers ?? []).some((provider) => provider.ready);
}

/**
 * Las paginas se acumulan: "cargar mas" agrega resultados al final en vez de
 * reemplazar los que ya estaban a la vista.
 */
export function useRemoteSearch(rawQuery: string, enabled: boolean) {
  const query = useDebouncedValue(rawQuery.trim(), REMOTE_SEARCH_DEBOUNCE_MS);
  const canSearch = enabled && query.length >= MIN_QUERY_LENGTH;

  return {
    query,
    canSearch,
    ...useInfiniteQuery({
      queryKey: queryKeys.remoteSearch(query),
      queryFn: ({ pageParam }) => ipc.searchRemoteSounds(query, pageParam),
      initialPageParam: 1,
      // Pedimos otra pagina mientras a algun proveedor le queden resultados.
      getNextPageParam: (lastPage, allPages) =>
        lastPage.some((result) => result.hasMore) ? allPages.length + 1 : undefined,
      enabled: canSearch,
      // Cache corto: las busquedas online se repiten mucho al volver atras (§13).
      staleTime: 2 * 60 * 1000,
      gcTime: 5 * 60 * 1000,
      retry: false,
      placeholderData: (previous) => previous,
    }),
  };
}
