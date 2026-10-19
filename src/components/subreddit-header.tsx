'use client';
import { useAuth } from '@/lib/auth-context';

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Users, Eye, Cake } from 'lucide-react';
import { api, Subreddit } from '@/lib/api';
import { SubredditAvatar } from './subreddit-avatar';
import { timeAgo, formatCount } from '@/lib/format';
import { cn } from '@/lib/utils';

interface SubredditHeaderProps {
  subreddit: Subreddit;
  onOpenCreatePost: () => void;
}

export function SubredditHeader({ subreddit, onOpenCreatePost }: SubredditHeaderProps) {
  const { user } = useAuth();
  const qc = useQueryClient();
  const [subscribed, setSubscribed] = useState(false);

  // Check subscription by listing user's subs
  useQuery({
    queryKey: ['my-subreddits'],
    queryFn: async () => {
      if (!user) return [];
      const list = await api.listSubreddits(100);
      return list.data.children.map((c) => c.data.display_name);
    },
    enabled: !!user,
    onSuccess: (subs: string[]) => {
      setSubscribed(subs.includes(subreddit.display_name));
    },
  });

  const subMut = useMutation({
    mutationFn: (action: 'sub' | 'unsub') => api.subscribe(subreddit.display_name, action),
    onSuccess: (_, action) => {
      setSubscribed(action === 'sub');
      qc.invalidateQueries({ queryKey: ['my-subreddits'] });
      qc.invalidateQueries({ queryKey: ['subreddits'] });
    },
  });

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden mb-3">
      {/* Banner */}
      <div
        className="h-20"
        style={{ backgroundColor: subreddit.primary_color || '#0079D3' }}
      />
      <div className="px-4 pb-4">
        <div className="flex items-end gap-3 -mt-6">
          <div className="rounded-full border-4 border-card bg-card">
            <SubredditAvatar name={subreddit.display_name} size={72} />
          </div>
          <div className="flex-1 pb-1">
            <h1 className="text-2xl font-bold">r/{subreddit.display_name}</h1>
            <p className="text-sm text-muted-foreground">r/{subreddit.display_name}</p>
          </div>
        </div>

        <div className="flex items-center gap-4 mt-3 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Users className="w-3.5 h-3.5" />
            <strong className="text-foreground">{formatCount(subreddit.subscribers)}</strong> Members
          </span>
          <span className="flex items-center gap-1">
            <Eye className="w-3.5 h-3.5" />
            <strong className="text-foreground">{formatCount(subreddit.active_user_count)}</strong> Online
          </span>
          <span className="flex items-center gap-1">
            <Cake className="w-3.5 h-3.5" />
            Created {timeAgo(subreddit.created_at)}
          </span>
        </div>

        <p className="mt-3 text-sm text-foreground/90">
          {subreddit.public_description || subreddit.description || 'No description.'}
        </p>

        <div className="mt-3 flex items-center gap-2">
          {user ? (
            <button
              onClick={() => subMut.mutate(subscribed ? 'unsub' : 'sub')}
              className={cn(
                'px-5 py-1.5 rounded-full text-sm font-semibold transition-colors',
                subscribed
                  ? 'bg-foreground/10 text-foreground hover:bg-foreground/15'
                  : 'bg-orange-500 hover:bg-orange-600 text-white'
              )}
              disabled={subMut.isLoading}
            >
              {subscribed ? 'Joined' : 'Join'}
            </button>
          ) : null}
          <button
            onClick={onOpenCreatePost}
            className="px-5 py-1.5 rounded-full text-sm font-semibold bg-foreground/10 text-foreground hover:bg-foreground/15 transition-colors"
          >
            Create Post
          </button>
          <a
            href={`#/r/${subreddit.display_name}/wiki`}
            className="px-5 py-1.5 rounded-full text-sm font-semibold bg-foreground/10 text-foreground hover:bg-foreground/15 transition-colors"
          >
            Wiki
          </a>
        </div>
      </div>
    </div>
  );
}
