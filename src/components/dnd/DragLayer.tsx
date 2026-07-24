import { GripVertical } from 'lucide-react';

import { useDragStore } from '@/features/dnd';

/**
 * Fantasma que sigue al cursor mientras se arrastra.
 *
 * Como el arrastre no usa la API HTML5, el navegador no dibuja ninguna
 * previsualizacion: la ponemos nosotros.
 */
export function DragLayer() {
  const payload = useDragStore((state) => state.payload);
  const label = useDragStore((state) => state.label);
  const position = useDragStore((state) => state.position);

  if (!payload || !position) return null;

  return (
    <div
      // `pointer-events-none` es imprescindible: si no, el fantasma taparia al
      // destino y `elementFromPoint` lo devolveria a el.
      className="pointer-events-none fixed z-[200] flex max-w-56 items-center gap-1.5 rounded-md border border-accent bg-surface-2 px-2 py-1 text-xs text-fg-default shadow-xl"
      style={{ left: position.x + 12, top: position.y + 12 }}
      aria-hidden
    >
      <GripVertical className="h-3 w-3 shrink-0 text-accent" />
      <span className="truncate">{label}</span>
    </div>
  );
}
