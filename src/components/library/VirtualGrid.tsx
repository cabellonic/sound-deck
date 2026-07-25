import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useRef, useState, type ReactNode } from 'react';

import { columnsForWidth } from './columns';

/** Alto estimado antes de medir; las filas reales pueden ser mas altas. */
const ROW_HEIGHT = 64;
const ROW_GAP = 6;

export interface VirtualGridProps<T> {
  items: T[];
  keyOf: (item: T) => string;
  renderItem: (item: T) => ReactNode;
  /** Se muestra cuando no hay elementos. */
  empty: ReactNode;
}

/**
 * Lista virtualizada que muestra entre una y tres columnas segun el ancho (§9).
 *
 * Virtualizamos por fila: con miles de sonidos solo se montan los visibles.
 */
export function VirtualGrid<T>({ items, keyOf, renderItem, empty }: VirtualGridProps<T>) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState(1);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;

    const update = (width: number) => setColumns(columnsForWidth(width));
    update(element.clientWidth);

    // `ResizeObserver` no existe en algunos entornos de test.
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) update(entry.contentRect.width);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const rows: T[][] = [];
  for (let index = 0; index < items.length; index += columns) {
    rows.push(items.slice(index, index + columns));
  }

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    // Solo una estimacion inicial: las filas crecen cuando el texto secundario
    // pasa a dos lineas, asi que el alto real lo mide `measureElement`.
    estimateSize: () => ROW_HEIGHT + ROW_GAP,
    overscan: 6,
  });

  // Al cambiar de una a dos columnas cambian los altos: hay que re-medir.
  useEffect(() => {
    virtualizer.measure();
  }, [columns, virtualizer]);

  return (
    <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto px-3 pb-3">
      {items.length === 0 ? (
        empty
      ) : (
        <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
          {virtualizer.getVirtualItems().map((virtualRow) => {
            const row = rows[virtualRow.index];
            if (!row) return null;

            return (
              <div
                key={virtualRow.key}
                // `data-index` + `measureElement` mantienen sincronizado el alto
                // medido con el indice de la fila.
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
                className="absolute left-0 top-0 grid w-full"
                style={{
                  transform: `translateY(${virtualRow.start}px)`,
                  gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                  gap: `${ROW_GAP}px`,
                  paddingBottom: `${ROW_GAP}px`,
                }}
              >
                {/* `h-full` iguala el alto de las dos columnas de la fila. */}
                {row.map((item) => (
                  <div key={keyOf(item)} className="[&>*]:h-full">
                    {renderItem(item)}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
