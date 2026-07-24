import '@testing-library/jest-dom/vitest';

import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';

// jsdom no implementa estas APIs y Radix las usa.
if (!window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })) as unknown as typeof window.matchMedia;
}

if (!window.ResizeObserver) {
  window.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
}

if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = vi.fn();
}

// `convertFileSrc` delega en los internals que inyecta el WebView de Tauri, que
// en jsdom no existen. Reproducimos la unica forma que nos importa: una ruta de
// disco convertida en una URL del protocolo `asset:`.
interface TauriInternals {
  convertFileSrc: (filePath: string, protocol?: string) => string;
}
const globalWithTauri = window as typeof window & { __TAURI_INTERNALS__?: TauriInternals };
if (!globalWithTauri.__TAURI_INTERNALS__) {
  globalWithTauri.__TAURI_INTERNALS__ = {
    convertFileSrc: (filePath, protocol = 'asset') =>
      `http://${protocol}.localhost/${encodeURIComponent(filePath)}`,
  };
}

// jsdom no implementa hit-testing; el drag por punteros lo usa para resolver el
// destino. Devuelve el body por defecto y los tests lo espian cuando importa.
if (!document.elementFromPoint) {
  document.elementFromPoint = (() => document.body) as typeof document.elementFromPoint;
}

if (!window.PointerEvent) {
  // jsdom vieja no trae PointerEvent; lo derivamos de MouseEvent.
  class PointerEventPolyfill extends MouseEvent {
    constructor(type: string, params: PointerEventInit = {}) {
      super(type, params);
    }
  }
  window.PointerEvent = PointerEventPolyfill as unknown as typeof PointerEvent;
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
