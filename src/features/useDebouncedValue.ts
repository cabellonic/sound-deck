import { useEffect, useState } from 'react';

/**
 * Valor retrasado.
 *
 * La busqueda online usa 300 ms (dentro del rango 250-400 ms de §13); la local
 * usa un retraso menor porque debe sentirse instantanea (§26).
 */
export function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    if (delayMs <= 0) {
      setDebounced(value);
      return;
    }

    const timer = window.setTimeout(() => setDebounced(value), delayMs);
    return () => window.clearTimeout(timer);
  }, [value, delayMs]);

  return debounced;
}
