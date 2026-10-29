'use client';

import { useQuery } from '@tanstack/react-query';
import { Shield, ScrollText, UserCog } from 'lucide-react';
import { api } from '@/lib/api';
import { UserAvatar } from './subreddit-avatar';
import { timeAgo, formatCount } from '@/lib/format';

interface SubredditSidebarProps {
  subredditName: string;
  onOpenUser: (name: string) => void;
}

export function SubredditSidebar({ subredditName, onOpenUser }: SubredditSidebarProps) {
  const { data: rulesRes } = useQuery({
    queryKey: ['subreddit-rules', subredditName],
    queryFn: () => api.listRules(subredditName),
  });
  const { data: modsRes } = useQuery({
    queryKey: ['subreddit-moderators', subredditName],
    queryFn: () => api.listModerators(subredditName),
  });

  const rules = rulesRes?.rules || [];
  const mods = modsRes?.moderators || [];

  return (
    <aside className="w-72 shrink-0 hidden xl:block sticky top-12 self-start h-[calc(100vh-3rem)] overflow-y-auto p-3 space-y-3">
      {rules.length > 0 && (
        <div className="bg-card border border-border rounded-lg overflow-hidden">
          <div className="bg-accent/40 px-3 py-2 font-semibold text-sm flex items-center gap-2">
            <ScrollText className="w-4 h-4" /> Rules
          </div>
          <ol className="divide-y divide-border">
            {rules.map((r, i) => (
              <li key={r.id} className="px-3 py-2 text-sm">
                <div className="font-semibold flex gap-2">
                  <span className="text-muted-foreground">{i + 1}.</span>
                  <span>{r.short_name}</span>
                </div>
                {r.description && <p className="text-xs text-muted-foreground mt-0.5 pl-5">{r.description}</p>}
              </li>
            ))}
          </ol>
        </div>
      )}

      {mods.length > 0 && (
        <div className="bg-card border border-border rounded-lg overflow-hidden">
          <div className="bg-accent/40 px-3 py-2 font-semibold text-sm flex items-center gap-2">
            <Shield className="w-4 h-4" /> Moderators
          </div>
          <ul className="divide-y divide-border">
            {mods.map((m) => (
              <li key={m.username}>
                <button
                  onClick={() => onOpenUser(m.username)}
                  className="w-full flex items-center gap-2 px-3 py-2 hover:bg-accent transition-colors text-left text-sm"
                >
                  <UserAvatar name={m.username} size={20} />
                  <span className="font-medium">u/{m.username}</span>
                  {m.permissions === 'all' && (
                    <span className="ml-auto text-xs text-muted-foreground bg-accent px-1.5 py-0.5 rounded">full</span>
                  )}
                </button>
              </li>
            ))}
            <li className="px-3 py-2 text-xs text-muted-foreground hover:bg-accent cursor-pointer">
              <UserCog className="w-3.5 h-3.5 inline mr-1" /> Message the mods
            </li>
          </ul>
        </div>
      )}

      <div className="text-xs text-muted-foreground px-2">
        Freeddit v0.1 — {formatCount(rules.length)} rules · {formatCount(mods.length)} mods
      </div>
    </aside>
  );
}
