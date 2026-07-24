import { useEffect } from 'react';

import type { ThemePreference } from '@/types/domain';

/**
 * Aplica el tema al elemento raiz.
 *
 * `system` sigue a la preferencia del sistema operativo y reacciona si cambia
 * mientras la aplicacion esta abierta.
 */
export function useTheme(preference: ThemePreference | undefined): void {
  useEffect(() => {
    if (!preference) return;

    const root = document.documentElement;
    const media = window.matchMedia?.('(prefers-color-scheme: light)');

    const apply = () => {
      const resolved = preference === 'system' ? (media?.matches ? 'light' : 'dark') : preference;
      root.dataset.theme = resolved;
      root.classList.toggle('dark', resolved === 'dark');
      root.style.colorScheme = resolved;
    };

    apply();

    if (preference !== 'system' || !media) return;
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [preference]);
}
