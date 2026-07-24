import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/Button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/primitives';
import { cn } from '@/lib/utils';
import { SLOT_NUMBERS, type PageSummary, type SlotNumber, type SoundPage } from '@/types/domain';

export interface AssignToSlotDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Nombre del audio que se va a asignar, para el titulo. */
  soundName: string;
  pages: PageSummary[];
  /** Pagina cargada para saber que slots estan ocupados. */
  currentPage: SoundPage | undefined;
  defaultPageId: string | null;
  onSelectPage: (pageId: string) => void;
  onConfirm: (pageId: string, slotNumber: SlotNumber) => void;
}

/**
 * Alternativa accesible al drag and drop (§25): elegir pagina y slot desde un
 * dialogo, operable enteramente con teclado.
 */
export function AssignToSlotDialog({
  open,
  onOpenChange,
  soundName,
  pages,
  currentPage,
  defaultPageId,
  onSelectPage,
  onConfirm,
}: AssignToSlotDialogProps) {
  const [pageId, setPageId] = useState<string | null>(defaultPageId);

  useEffect(() => {
    if (open) setPageId(defaultPageId);
  }, [open, defaultPageId]);

  const occupied = new Set(
    (currentPage?.id === pageId ? (currentPage?.slots ?? []) : [])
      .filter((slot) => slot.sound !== null)
      .map((slot) => slot.slotNumber),
  );

  const handlePageChange = (value: string) => {
    setPageId(value);
    onSelectPage(value);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader
          title="Asignar a un boton"
          description={`Elegi la pagina y el boton para "${soundName}".`}
        />

        <div className="flex flex-col gap-4 px-5 py-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium text-fg-default">Pagina</span>
            <Select value={pageId ?? undefined} onValueChange={handlePageChange}>
              <SelectTrigger aria-label="Pagina de destino">
                <SelectValue placeholder="Elegi una pagina" />
              </SelectTrigger>
              <SelectContent>
                {pages.map((page) => (
                  <SelectItem key={page.id} value={page.id}>
                    {page.name} ({page.assignedSlots}/9)
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-sm font-medium text-fg-default">Boton</span>
            <div className="grid grid-cols-3 gap-2" role="group" aria-label="Boton de destino">
              {SLOT_NUMBERS.map((slotNumber) => {
                const taken = occupied.has(slotNumber);
                return (
                  <button
                    key={slotNumber}
                    type="button"
                    disabled={!pageId}
                    onClick={() => {
                      if (pageId) {
                        onConfirm(pageId, slotNumber);
                        onOpenChange(false);
                      }
                    }}
                    className={cn(
                      'flex h-14 flex-col items-center justify-center rounded-md border text-sm transition-colors',
                      'disabled:cursor-not-allowed disabled:opacity-50',
                      taken
                        ? 'border-border-strong bg-surface-2 hover:border-accent'
                        : 'border-dashed border-border-subtle bg-surface-1 hover:border-accent',
                    )}
                  >
                    <span className="font-mono font-semibold">{slotNumber}</span>
                    <span className="text-[10px] text-fg-subtle">
                      {taken ? 'Ocupado' : 'Libre'}
                    </span>
                  </button>
                );
              })}
            </div>
            <p className="text-xs text-fg-subtle">
              Elegir un boton ocupado reemplaza su asignacion actual.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
