'use client';

import { Flame, Sparkles, TrendingUp, Clock, ChartColumn, Zap, Swords } from 'lucide-react';
import { PostSort } from '@/lib/api';
import { cn } from '@/lib/utils';

interface SortTabsProps {
  value: PostSort;
  onChange: (sort: PostSort) => void;
  showTimeFilter?: boolean;
  timeFilter?: 'hour' | 'day' | 'week' | 'month' | 'year' | 'all';
  onTimeFilterChange?: (t: SortTabsProps['timeFilter']) => void;
}

const SORTS: { value: PostSort; label: string; icon: React.ReactNode }[] = [
  { value: 'hot', label: 'Hot', icon: <Flame className="w-4 h-4" /> },
  { value: 'best', label: 'Best', icon: <Sparkles className="w-4 h-4" /> },
  { value: 'new', label: 'New', icon: <Clock className="w-4 h-4" /> },
  { value: 'top', label: 'Top', icon: <ChartColumn className="w-4 h-4" /> },
  { value: 'rising', label: 'Rising', icon: <TrendingUp className="w-4 h-4" /> },
  { value: 'controversial', label: 'Controversial', icon: <Swords className="w-4 h-4" /> },
];

const TIME_FILTERS = ['hour', 'day', 'week', 'month', 'year', 'all'] as const;

export function SortTabs({ value, onChange, showTimeFilter, timeFilter = 'all', onTimeFilterChange }: SortTabsProps) {
  return (
    <div className="flex items-center gap-1 bg-card border border-border rounded-lg p-1 overflow-x-auto">
      {SORTS.map((s) => (
        <button
          key={s.value}
          onClick={() => onChange(s.value)}
          className={cn(
            'flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-semibold transition-colors whitespace-nowrap',
            value === s.value
              ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400'
              : 'text-muted-foreground hover:bg-accent hover:text-foreground'
          )}
        >
          {s.icon}
          {s.label}
        </button>
      ))}
      {showTimeFilter && value === 'top' && onTimeFilterChange && (
        <select
          value={timeFilter}
          onChange={(e) => onTimeFilterChange(e.target.value as typeof timeFilter)}
          className="ml-2 bg-accent/40 border border-border rounded-md px-2 py-1.5 text-sm"
        >
          {TIME_FILTERS.map((t) => (
            <option key={t} value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
          ))}
        </select>
      )}
    </div>
  );
}
