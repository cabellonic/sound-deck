/** Anchos a partir de los cuales entra otra columna sin apretar las filas. */
const TWO_COLUMN_BREAKPOINT = 620;
const THREE_COLUMN_BREAKPOINT = 980;

/**
 * Cuantas columnas entran en un ancho dado.
 *
 * Tres es el maximo: con la ventana maximizada en 1920 la biblioteca queda con
 * ~1550px, y a partir de la cuarta columna el nombre del audio deja de entrar.
 */
export function columnsForWidth(width: number): number {
  if (width >= THREE_COLUMN_BREAKPOINT) return 3;
  if (width >= TWO_COLUMN_BREAKPOINT) return 2;
  return 1;
}
