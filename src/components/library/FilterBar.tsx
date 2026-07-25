import { Clock, Filter, Flame, LayoutGrid, Unlink } from 'lucide-react';

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/primitives';
import { Button } from '@/components/ui/Button';
import { categoryKey, type PlainKey } from '@/i18n';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import {
  type LibraryFacets,
  type LibraryFilter,
  type NormalizedCategory,
  type SoundSortOrder,
} from '@/types/domain';

const SORT_LABELS: Record<SoundSortOrder, PlainKey> = {
  relevance: 'sort.relevance',
  recent: 'sort.recent',
  most_played: 'sort.mostPlayed',
  name: 'sort.name',
};

/** Filtros rapidos siempre presentes. */
const QUICK_FILTERS: Array<{
  filter: LibraryFilter;
  label: PlainKey;
  icon: typeof LayoutGrid;
}> = [
  { filter: { type: 'all' }, label: 'filter.all', icon: LayoutGrid },
  { filter: { type: 'recent' }, label: 'filter.recent', icon: Clock },
  { filter: { type: 'most_played' }, label: 'filter.mostPlayed', icon: Flame },
  { filter: { type: 'unassigned' }, label: 'filter.unassigned', icon: Unlink },
];

function sameFilter(a: LibraryFilter, b: LibraryFilter): boolean {
  if (a.type !== b.type) return false;
  if (a.type === 'category' && b.type === 'category') return a.category === b.category;
  if (a.type === 'provider' && b.type === 'provider') return a.providerId === b.providerId;
  return true;
}

export interface FilterBarProps {
  filter: LibraryFilter;
  onFilterChange: (filter: LibraryFilter) => void;
  sort: SoundSortOrder;
  onSortChange: (sort: SoundSortOrder) => void;
  facets: LibraryFacets | undefined;
}

/**
 * Filtros automaticos de la biblioteca (§9).
 *
 * Las categorias y proveedores solo aparecen si existen sonidos que los usan.
 */
export function FilterBar({ filter, onFilterChange, sort, onSortChange, facets }: FilterBarProps) {
  const { t } = useTranslation();
  const categories = (facets?.categories ?? []).filter(
    (facet) => facet.value !== 'uncategorized' && facet.count > 0,
  );
  const uncategorized = facets?.categories.find((facet) => facet.value === 'uncategorized');
  const providers = facets?.providers ?? [];
  const hasDynamicFilters = categories.length > 0 || providers.length > 0 || Boolean(uncategorized);

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {QUICK_FILTERS.map(({ filter: quickFilter, label, icon: Icon }) => (
        <button
          key={label}
          type="button"
          onClick={() => onFilterChange(quickFilter)}
          aria-pressed={sameFilter(filter, quickFilter)}
          className={cn(
            'inline-flex h-6.5 items-center gap-1.5 rounded-md border px-2 py-1 text-xs transition-colors',
            sameFilter(filter, quickFilter)
              ? 'border-accent bg-accent-soft text-fg-default'
              : 'border-border-subtle text-fg-muted hover:bg-surface-2 hover:text-fg-default',
          )}
        >
          <Icon className="h-3 w-3" aria-hidden />
          {t(label)}
        </button>
      ))}

      {hasDynamicFilters ? (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="sm" variant="ghost" aria-label={t('filter.moreLabel')}>
              <Filter className="h-3 w-3" aria-hidden />
              {t('filter.more')}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {categories.length > 0 ? (
              <>
                <DropdownMenuLabel>{t('filter.categories')}</DropdownMenuLabel>
                {categories.map((facet) => (
                  <DropdownMenuItem
                    key={facet.value}
                    onSelect={() =>
                      onFilterChange({
                        type: 'category',
                        category: facet.value as NormalizedCategory,
                      })
                    }
                  >
                    {t(categoryKey(facet.value))}
                    <span className="ml-auto font-mono text-xs text-fg-subtle">{facet.count}</span>
                  </DropdownMenuItem>
                ))}
              </>
            ) : null}

            {uncategorized && uncategorized.count > 0 ? (
              <DropdownMenuItem onSelect={() => onFilterChange({ type: 'uncategorized' })}>
                {t('category.uncategorized')}
                <span className="ml-auto font-mono text-xs text-fg-subtle">
                  {uncategorized.count}
                </span>
              </DropdownMenuItem>
            ) : null}

            {providers.length > 0 ? (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel>{t('filter.providers')}</DropdownMenuLabel>
                {providers.map((facet) => (
                  <DropdownMenuItem
                    key={facet.value}
                    onSelect={() => onFilterChange({ type: 'provider', providerId: facet.value })}
                  >
                    <span className="capitalize">{facet.value}</span>
                    <span className="ml-auto font-mono text-xs text-fg-subtle">{facet.count}</span>
                  </DropdownMenuItem>
                ))}
              </>
            ) : null}
          </DropdownMenuContent>
        </DropdownMenu>
      ) : null}

      <div className="ml-auto">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              size="sm"
              variant="ghost"
              aria-label={t('filter.sortByValue', { value: t(SORT_LABELS[sort]) })}
            >
              {t(SORT_LABELS[sort])}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>{t('filter.sortBy')}</DropdownMenuLabel>
            {(Object.keys(SORT_LABELS) as SoundSortOrder[]).map((option) => (
              <DropdownMenuItem key={option} onSelect={() => onSortChange(option)}>
                {t(SORT_LABELS[option])}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
