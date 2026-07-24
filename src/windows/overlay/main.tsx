import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import '@/styles/globals.css';

import { initFocusRing } from '@/lib/focusRing';

import { OverlayApp } from './OverlayApp';

initFocusRing();

// El overlay no usa TanStack Query: su bundle debe ser lo mas chico posible
// para que mostrarlo sea instantaneo (§16).
const container = document.getElementById('root');
if (!container) {
  throw new Error('No se encontro el contenedor #root');
}

createRoot(container).render(
  <StrictMode>
    <OverlayApp />
  </StrictMode>,
);
