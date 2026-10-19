'use client';

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search as SearchIcon } from 'lucide-react';
import { api, Post, Subreddit, User } from '@/lib/api';
import { SubredditAvatar, UserAvatar } from './subreddit-avatar';
import { PostCard } from './post-card';
import { formatCount } from '@/lib/format';
import { cn } from '@/lib/utils';

interface SearchPageProps {
  query: string;
  onOpenPost: (post: Post) => void;
  onOpenSubreddit: (name: string) => void;
  onOpenUser: (name: string) => void;
}

export function SearchPage({ query, onOpenPost, onOpenSubreddit, onOpenUser }: SearchPageProps) {
  const [type, setType] = useState<'all' | 'link' | 'sr' | 'user'>('all');
  const { data, isLoading } = useQuery({
    queryKey: ['search', query, type],
    queryFn: () => api.search(query, type === 'all' ? undefined : (type as 'link' | 'sr' | 'user')),
    enabled: !!query,
  });

  const posts = data?.data.children.filter((c) => c.kind === 't3').map((c) => c.data as Post) || [];
  const subs = data?.data.children.filter((c) => c.kind === 't5').map((c) => c.data as Subreddit) || [];
  const users = data?.data.children.filter((c) => c.kind === 't2').map((c) => c.data as User) || [];

  return (
    <div className="max-w-3xl mx-auto">
      <div className="bg-card border border-border rounded-lg p-3 mb-3 flex items-center gap-2">
        <SearchIcon className="w-5 h-5 text-muted-foreground" />
        <h1 className="text-lg font-semibold">Search results for "{query}"</h1>
      </div>

      <div className="flex gap-1 mb-3 bg-card border border-border rounded-lg p-1">
        {(['all', 'link', 'sr', 'user'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setType(t)}
            className={cn(
              'flex-1 py-1.5 rounded-md text-sm font-semibold capitalize',
              type === t ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'text-muted-foreground hover:bg-accent'
            )}
          >
            {t === 'all' ? 'All' : t === 'link' ? 'Posts' : t === 'sr' ? 'Communities' : 'People'}
          </button>
        ))}
      </div>

      {isLoading ? (
        <div className="space-y-2">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="h-24 bg-accent/30 animate-pulse rounded" />
          ))}
        </div>
      ) : !data || data.data.children.length === 0 ? (
        <div className="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
          <p>No results found for "{query}".</p>
        </div>
      ) : (
        <div className="space-y-3">
          {subs.length > 0 && (type === 'all' || type === 'sr') && (
            <div className="space-y-2">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">Communities</h3>
              {subs.map((s) => (
                <button
                  key={s.id}
                  onClick={() => onOpenSubreddit(s.display_name)}
                  className="w-full flex items-center gap-3 bg-card border border-border rounded-lg p-3 hover:border-foreground/20 transition-colors text-left"
                >
                  <SubredditAvatar name={s.display_name} size={40} />
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold">r/{s.display_name}</div>
                    <div className="text-xs text-muted-foreground">
                      {formatCount(s.subscribers)} members • {formatCount(s.active_user_count)} online
                    </div>
                    {s.public_description && (
                      <p className="text-sm text-muted-foreground mt-1 line-clamp-1">{s.public_description}</p>
                    )}
                  </div>
                </button>
              ))}
            </div>
          )}

          {users.length > 0 && (type === 'all' || type === 'user') && (
            <div className="space-y-2">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">People</h3>
              {users.map((u) => (
                <button
                  key={u.id}
                  onClick={() => onOpenUser(u.name)}
                  className="w-full flex items-center gap-3 bg-card border border-border rounded-lg p-3 hover:border-foreground/20 transition-colors text-left"
                >
                  <UserAvatar name={u.name} size={40} />
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold">u/{u.name}</div>
                    <div className="text-xs text-muted-foreground">
                      {formatCount(u.post_karma + u.comment_karma)} karma
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}

          {posts.length > 0 && (type === 'all' || type === 'link') && (
            <div className="space-y-2">
              <h3 className="text-xs font-bold uppercase tracking-wider text-muted-foreground">Posts</h3>
              {posts.map((p) => (
                <PostCard
                  key={p.id}
                  post={p}
                  onOpenPost={onOpenPost}
                  onOpenSubreddit={onOpenSubreddit}
                  onOpenUser={onOpenUser}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
