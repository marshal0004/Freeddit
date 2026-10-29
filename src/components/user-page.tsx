'use client';
import { useAuth } from '@/lib/auth-context';

import { useState } from 'react';
import { Cake, Award, Link as LinkIcon } from 'lucide-react';
import { api, User, Post, Comment } from '@/lib/api';
import { useQuery } from '@tanstack/react-query';
import { UserAvatar } from './subreddit-avatar';
import { PostCard } from './post-card';
import { VoteButton } from './vote-button';
import { timeAgo, formatCount } from '@/lib/format';
import { cn } from '@/lib/utils';

interface UserPageProps {
  username: string;
  onOpenPost: (post: Post) => void;
  onOpenSubreddit: (name: string) => void;
  onOpenUser: (name: string) => void;
}

export function UserPage({ username, onOpenPost, onOpenSubreddit, onOpenUser }: UserPageProps) {
  const [tab, setTab] = useState<'overview' | 'posts' | 'comments'>('overview');
  const { data: userRes } = useQuery({
    queryKey: ['user', username],
    queryFn: () => api.getUser(username),
  });
  const { data: posts } = useQuery({
    queryKey: ['user', username, 'posts'],
    queryFn: () => api.getUserPosts(username, 'new', 50),
  });
  const { data: comments } = useQuery({
    queryKey: ['user', username, 'comments'],
    queryFn: () => api.getUserComments(username, 'new', 50),
  });

  const user: User | undefined = userRes?.data;
  if (!user) return null;

  return (
    <div className="max-w-3xl mx-auto">
      <div className="bg-card border border-border rounded-lg overflow-hidden mb-3">
        <div
          className="h-24"
          style={{ backgroundColor: user.banner_color || '#1a1a1b' }}
        />
        <div className="px-4 pb-4">
          <div className="flex items-end gap-3 -mt-8">
            <div className="rounded-full border-4 border-card bg-card">
              <UserAvatar name={user.name} size={80} />
            </div>
            <div className="flex-1 pb-2">
              <h1 className="text-2xl font-bold">{user.name}</h1>
              <p className="text-sm text-muted-foreground">u/{user.name}</p>
            </div>
          </div>
          <div className="flex items-center gap-4 mt-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Cake className="w-3.5 h-3.5" />
              Cake day {new Date(user.created_at).toLocaleDateString()}
            </span>
            <span className="flex items-center gap-1">
              <Award className="w-3.5 h-3.5" />
              <strong className="text-foreground">{formatCount(user.post_karma + user.comment_karma)}</strong> karma
            </span>
          </div>
          {user.bio && <p className="mt-3 text-sm text-foreground/90">{user.bio}</p>}
        </div>
      </div>

      <div className="flex gap-1 mb-3 bg-card border border-border rounded-lg p-1">
        {(['overview', 'posts', 'comments'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={cn(
              'flex-1 py-1.5 rounded-md text-sm font-semibold capitalize',
              tab === t ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'text-muted-foreground hover:bg-accent'
            )}
          >
            {t}
          </button>
        ))}
      </div>

      <div className="space-y-3">
        {tab === 'overview' && (
          <>
            {posts?.data.children.map(({ data: p }) => (
              <PostCard
                key={p.id}
                post={p}
                onOpenPost={onOpenPost}
                onOpenSubreddit={onOpenSubreddit}
                onOpenUser={onOpenUser}
              />
            ))}
            {comments?.data.children.map(({ data: c }) => (
              <div key={c.id} className="bg-card border border-border rounded-lg p-3">
                <p className="text-xs text-muted-foreground mb-1">
                  Comment in{' '}
                  <button
                    onClick={() => onOpenSubreddit(c.subreddit)}
                    className="font-semibold hover:underline text-foreground"
                  >r/{c.subreddit}</button>
                  {' '}• {timeAgo(c.created_at)}
                </p>
                <p className="text-sm">{c.body}</p>
                <div className="flex items-center gap-2 mt-2">
                  <VoteButton targetId={c.name} score={c.score} initialLikes={c.likes} variant="comment" />
                  <span className="text-xs text-muted-foreground">{formatCount(c.num_replies)} replies</span>
                </div>
              </div>
            ))}
          </>
        )}
        {tab === 'posts' && posts?.data.children.map(({ data: p }) => (
          <PostCard
            key={p.id}
            post={p}
            onOpenPost={onOpenPost}
            onOpenSubreddit={onOpenSubreddit}
            onOpenUser={onOpenUser}
          />
        ))}
        {tab === 'comments' && comments?.data.children.map(({ data: c }) => (
          <div key={c.id} className="bg-card border border-border rounded-lg p-3">
            <p className="text-xs text-muted-foreground mb-1">
              Comment in{' '}
              <button
                onClick={() => onOpenSubreddit(c.subreddit)}
                className="font-semibold hover:underline text-foreground"
              >r/{c.subreddit}</button>
              {' '}• {timeAgo(c.created_at)}
            </p>
            <p className="text-sm">{c.body}</p>
            <div className="flex items-center gap-2 mt-2">
              <VoteButton targetId={c.name} score={c.score} initialLikes={c.likes} variant="comment" />
              <span className="text-xs text-muted-foreground">{formatCount(c.num_replies)} replies</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
