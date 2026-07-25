import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';

import { TooltipProvider } from '@/components/ui/primitives';

/**
 * Envoltorio con el contexto minimo que la aplicacion siempre tiene montado.
 *
 * Los componentes traducen con `useTranslation`, que lee la configuracion por
 * TanStack Query: sin este proveedor, cualquier render de prueba explota por un
 * motivo que no tiene nada que ver con lo que se esta probando.
 */
export function renderWithProviders(ui: ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={client}>
        <TooltipProvider>{children}</TooltipProvider>
      </QueryClientProvider>
    );
  }

  return render(ui, { wrapper: Wrapper });
}
