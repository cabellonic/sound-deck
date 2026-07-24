import { useEffect } from 'react';

import { isEditableTarget } from '@/lib/utils';

/**
 * Suprime el menu contextual del webview.
 *
 * Sin esto, el clic derecho abre el menu del navegador (recargar, inspeccionar,
 * guardar imagen...), que no tiene sentido en una aplicacion de escritorio y se
 * superpone con nuestros propios menus contextuales.
 *
 * Se mantiene en los campos de texto, donde copiar y pegar si hacen falta.
 */
export function useNativeContextMenu(): void {
  useEffect(() => {
    const handler = (event: MouseEvent) => {
      if (isEditableTarget(event.target)) return;
      event.preventDefault();
    };

    document.addEventListener('contextmenu', handler);
    return () => document.removeEventListener('contextmenu', handler);
  }, []);
}
