'use client';
import { useAuth } from '@/lib/auth-context';

import { useQuery } from '@tanstack/react-query';
import { Home, Compass, Plus, TrendingUp, Users } from 'lucide-react';
import { api } from '@/lib/api';
import { SubredditAvatar } from './subreddit-avatar';
import { cn } from '@/lib/utils';
import { formatCount } from '@/lib/format';

interface SidebarProps {
  onOpenHome: () => void;
  onOpenPopular: () => void;
  onOpenSubreddit: (name: string) => void;
  onCreateSubreddit: () => void;
  onClose?: () => void;
}

export function Sidebar({ onOpenHome, onOpenPopular, onOpenSubreddit, onCreateSubreddit }: SidebarProps) {
  const { user } = useAuth();
  const { data: popular } = useQuery({
    queryKey: ['subreddits', 'popular'],
    queryFn: () => api.popularSubreddits(10),
  });

  const recentSubs = user ? (popular?.data.children.slice(0, 5) || []) : [];

  return (
    <aside className="w-64 shrink-0 hidden lg:block sticky top-12 self-start h-[calc(100vh-3rem)] overflow-y-auto p-3">
      <nav className="space-y-1">
        <button
          onClick={onOpenHome}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-accent transition-colors text-sm font-semibold"
        >
          <Home className="w-5 h-5" /> Home
        </button>
        <button
          onClick={onOpenPopular}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-accent transition-colors text-sm font-semibold"
        >
          <Compass className="w-5 h-5" /> Popular
        </button>
        <button
          onClick={onCreateSubreddit}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-md hover:bg-accent transition-colors text-sm font-semibold"
        >
          <Plus className="w-5 h-5" /> Create a community
        </button>
      </nav>

      {recentSubs.length > 0 && (
        <div className="mt-6">
          <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground px-3 mb-1">
            Recent
          </h3>
          <nav className="space-y-0.5">
            {recentSubs.map(({ data: sub }) => (
              <button
                key={sub.id}
                onClick={() => onOpenSubreddit(sub.display_name)}
                className="w-full flex items-center gap-2 px-3 py-1.5 rounded-md hover:bg-accent transition-colors text-sm text-left"
              >
                <SubredditAvatar name={sub.display_name} size={20} />
                <span className="truncate">r/{sub.display_name}</span>
              </button>
            ))}
          </nav>
        </div>
      )}

      <div className="mt-6">
        <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground px-3 mb-1 flex items-center gap-1">
          <TrendingUp className="w-3 h-3" /> Popular Communities
        </h3>
        <nav className="space-y-0.5">
          {popular?.data.children.slice(0, 10).map(({ data: sub }, i) => (
            <button
              key={sub.id}
              onClick={() => onOpenSubreddit(sub.display_name)}
              className="w-full flex items-center gap-2 px-3 py-1.5 rounded-md hover:bg-accent transition-colors text-sm text-left"
            >
              <span className="text-muted-foreground w-4 text-right">{i + 1}</span>
              <SubredditAvatar name={sub.display_name} size={20} />
              <div className="min-w-0">
                <div className="truncate font-medium">r/{sub.display_name}</div>
                <div className="text-xs text-muted-foreground">{formatCount(sub.subscribers)} members</div>
              </div>
            </button>
          ))}
        </nav>
      </div>

      <div className="mt-6 px-3 text-xs text-muted-foreground space-y-2">
        <p>Freeddit v0.1 — Rust backend + Next.js frontend, faithful Reddit clone.</p>
        <p>Ranking algorithms (Hot, Best, Top, New, Rising, Controversial) implemented from Reddit's published source.</p>
      </div>
    </aside>
  );
}
