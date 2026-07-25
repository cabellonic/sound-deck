import { ChevronLeft, ChevronRight, Copy, MoreVertical, Pencil, Plus, Trash2 } from 'lucide-react';

import { Button } from '@/components/ui/Button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Tooltip,
} from '@/components/ui/primitives';
import { isClickSuppressed, useDragSource, useDropTarget } from '@/features/dnd';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import { MAX_PAGES, SLOTS_PER_PAGE, type PageSummary } from '@/types/domain';

/** Pestana de una pagina: se puede seleccionar y arrastrar para reordenar. */
function PageTab({
  page,
  isActive,
  onSelect,
  onDropPage,
}: {
  page: PageSummary;
  isActive: boolean;
  onSelect: (pageId: string) => void;
  onDropPage: (sourceId: string, targetId: string) => void;
}) {
  const { t } = useTranslation();
  const { onPointerDown } = useDragSource(() => ({ kind: 'page', pageId: page.id }), page.name);
  const { dropProps, isOver } = useDropTarget(`page:${page.id}`, (payload) => {
    if (payload.kind === 'page') onDropPage(payload.pageId, page.id);
  });

  return (
    <button
      {...dropProps}
      type="button"
      role="tab"
      aria-selected={isActive}
      onPointerDown={onPointerDown}
      onClick={() => {
        if (isClickSuppressed()) return;
        onSelect(page.id);
      }}
      className={cn(
        'flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-xs font-medium transition-colors',
        isActive
          ? 'border-accent bg-accent-soft text-fg-default'
          : 'border-border-subtle bg-surface-1 text-fg-muted hover:bg-surface-2 hover:text-fg-default',
        isOver && 'border-accent ring-2 ring-accent',
      )}
    >
      <span className="max-w-32 truncate">{page.name}</span>
      <span
        className="font-mono text-[10px] tabular-nums text-fg-subtle"
        aria-label={t('soundboard.pageSlots', {
          page: page.name,
          assigned: page.assignedSlots,
          total: SLOTS_PER_PAGE,
        })}
      >
        {page.assignedSlots}/9
      </span>
    </button>
  );
}

export interface PageBarProps {
  pages: PageSummary[];
  activePageId: string | null;
  onSelect: (pageId: string) => void;
  onCreate: () => void;
  onRename: (page: PageSummary) => void;
  onDelete: (page: PageSummary) => void;
  onDuplicate: (page: PageSummary) => void;
  onReorder: (pageIds: string[]) => void;
}

/** Selector de paginas con reordenamiento por arrastre (§8). */
export function PageBar({
  pages,
  activePageId,
  onSelect,
  onCreate,
  onRename,
  onDelete,
  onDuplicate,
  onReorder,
}: PageBarProps) {
  const { t } = useTranslation();
  const activeIndex = pages.findIndex((page) => page.id === activePageId);
  const goRelative = (delta: number) => {
    if (pages.length === 0) return;
    const base = activeIndex >= 0 ? activeIndex : 0;
    const next = (base + delta + pages.length) % pages.length;
    const target = pages[next];
    if (target) onSelect(target.id);
  };

  /** Coloca la pagina arrastrada justo antes de aquella sobre la que se solto. */
  const movePageBefore = (sourceId: string, targetId: string) => {
    if (sourceId === targetId) return;

    const ordered = pages.map((page) => page.id).filter((id) => id !== sourceId);
    const targetIndex = ordered.indexOf(targetId);
    ordered.splice(targetIndex < 0 ? ordered.length : targetIndex, 0, sourceId);
    onReorder(ordered);
  };

  const activePage = activeIndex >= 0 ? pages[activeIndex] : undefined;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-1.5">
        <h2 className="mr-auto text-xs font-semibold uppercase tracking-wide text-fg-subtle">
          {t('soundboard.title')}
        </h2>

        <Tooltip content={t('soundboard.newPage', { max: MAX_PAGES })}>
          <Button
            size="icon"
            variant="ghost"
            onClick={onCreate}
            disabled={pages.length >= MAX_PAGES}
            aria-label={t('soundboard.createPage')}
          >
            <Plus className="h-4 w-4" aria-hidden />
          </Button>
        </Tooltip>

        {activePage ? (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" aria-label={t('soundboard.pageActions')}>
                <MoreVertical className="h-4 w-4" aria-hidden />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => onRename(activePage)}>
                <Pencil className="h-3.5 w-3.5" aria-hidden />
                {t('soundboard.renamePage')}
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() => onDuplicate(activePage)}
                disabled={pages.length >= MAX_PAGES}
              >
                <Copy className="h-3.5 w-3.5" aria-hidden />
                {t('soundboard.duplicatePage')}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                destructive
                onSelect={() => onDelete(activePage)}
                disabled={pages.length <= 1}
              >
                <Trash2 className="h-3.5 w-3.5" aria-hidden />
                {t('soundboard.deletePage')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ) : null}
      </div>

      <div className="flex flex-wrap gap-1" role="tablist" aria-label={t('soundboard.pages')}>
        {pages.map((page) => (
          <PageTab
            key={page.id}
            page={page}
            isActive={page.id === activePageId}
            onSelect={onSelect}
            onDropPage={movePageBefore}
          />
        ))}
      </div>

      <div className="flex items-center justify-center gap-3 text-xs text-fg-muted">
        <Button
          size="icon"
          variant="ghost"
          onClick={() => goRelative(-1)}
          disabled={pages.length <= 1}
          aria-label={t('soundboard.previousPage')}
        >
          <ChevronLeft className="h-4 w-4" aria-hidden />
        </Button>
        <span className="font-mono tabular-nums" aria-live="polite">
          {activeIndex >= 0 ? activeIndex + 1 : 0} / {pages.length}
        </span>
        <Button
          size="icon"
          variant="ghost"
          onClick={() => goRelative(1)}
          disabled={pages.length <= 1}
          aria-label={t('soundboard.nextPage')}
        >
          <ChevronRight className="h-4 w-4" aria-hidden />
        </Button>
      </div>
    </div>
  );
}
