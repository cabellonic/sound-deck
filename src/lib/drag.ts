/**
 * Payloads de drag and drop (§25).
 *
 * El arrastre interno se resuelve con eventos de puntero (ver `features/dnd.ts`),
 * no con la API HTML5, porque esta ultima esta desactivada en Windows cuando la
 * ventana acepta el drop nativo de archivos de Tauri. Aca solo vive la forma del
 * payload: identificadores, nunca objetos pesados. Los resultados online
 * completos siguen en el cache del backend, que resuelve por `providerId` +
 * `remoteId` cuando llega el momento de descargar.
 */
import type { SlotNumber } from '@/types/domain';

export type DragPayload =
  | { kind: 'local-sound'; soundId: string }
  | { kind: 'remote-sound'; providerId: string; remoteId: string }
  | { kind: 'slot'; pageId: string; slotNumber: SlotNumber }
  | { kind: 'page'; pageId: string };

/** Valida la forma de un payload. Rechaza tipos desconocidos o incompletos. */
export function isDragPayload(value: unknown): value is DragPayload {
  if (typeof value !== 'object' || value === null || !('kind' in value)) return false;

  const candidate = value as Record<string, unknown>;
  switch (candidate.kind) {
    case 'local-sound':
      return typeof candidate.soundId === 'string' && candidate.soundId.length > 0;
    case 'remote-sound':
      return typeof candidate.providerId === 'string' && typeof candidate.remoteId === 'string';
    case 'slot':
      return (
        typeof candidate.pageId === 'string' &&
        typeof candidate.slotNumber === 'number' &&
        Number.isInteger(candidate.slotNumber) &&
        candidate.slotNumber >= 1 &&
        candidate.slotNumber <= 9
      );
    case 'page':
      return typeof candidate.pageId === 'string' && candidate.pageId.length > 0;
    default:
      return false;
  }
}
