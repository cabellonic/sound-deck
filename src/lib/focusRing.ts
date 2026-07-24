/**
 * Modo de navegacion para el anillo de foco (§28).
 *
 * Chromium considera `:focus-visible` a cualquier elemento que haya quedado
 * enfocado con un click apenas se pulsa una tecla. Con los atajos 1-9 eso hacia
 * aparecer el anillo en el ultimo lugar tocado con el mouse, que no tiene nada
 * que ver con lo que se esta haciendo. Anotamos con que se esta navegando y el
 * CSS solo dibuja el anillo cuando el foco lo mueve el teclado.
 */

/** Teclas que mueven el foco; el resto son atajos y no cambian el modo. */
const NAVIGATION_KEYS = new Set([
  'Tab',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'Home',
  'End',
]);

export type NavigationMode = 'pointer' | 'keyboard';

/** Deja de escuchar. Solo lo usan los tests. */
export type StopFocusRing = () => void;

export function initFocusRing(): StopFocusRing {
  const root = document.documentElement;
  const setMode = (mode: NavigationMode) => {
    root.dataset.nav = mode;
  };

  // Arrancamos en modo mouse: hasta que alguien tabule no hay nada que marcar.
  setMode('pointer');

  const onKeyDown = (event: KeyboardEvent) => {
    if (NAVIGATION_KEYS.has(event.key)) setMode('keyboard');
  };
  const onPointerDown = () => setMode('pointer');

  // En captura, porque los atajos y el arrastre detienen la propagacion.
  window.addEventListener('keydown', onKeyDown, true);
  window.addEventListener('pointerdown', onPointerDown, true);

  return () => {
    window.removeEventListener('keydown', onKeyDown, true);
    window.removeEventListener('pointerdown', onPointerDown, true);
  };
}
