'use client';

import { useState } from 'react';
import Link from 'next/link';
import { MessageSquare, Share2, Bookmark, MoreHorizontal, Link2, FileText, Image as ImageIcon, EyeOff, Flag } from 'lucide-react';
import { Post, api } from '@/lib/api';
import { useAuth } from '@/lib/auth-context';
import { timeAgo, formatCount, colorFromString, initials } from '@/lib/format';
import { VoteButton } from './vote-button';
import { cn } from '@/lib/utils';
import { SubredditAvatar } from './subreddit-avatar';
import { Markdown } from './markdown';

interface PostCardProps {
  post: Post;
  onOpenPost?: (post: Post) => void;
  onOpenSubreddit?: (name: string) => void;
  onOpenUser?: (name: string) => void;
}

export function PostCard({ post, onOpenPost, onOpenSubreddit, onOpenUser }: PostCardProps) {
  const [imageError, setImageError] = useState(false);
  const [saved, setSaved] = useState(false);
  const [hidden, setHidden] = useState(false);
  const { user } = useAuth();

  const isLink = post.kind === 'link' && post.url;
  const isSelf = post.is_self || post.kind === 'self';
  const domain = isLink && post.url ? (() => {
    try { return new URL(post.url).hostname.replace(/^www\./, ''); } catch { return null; }
  })() : null;

  return (
    <article
      className="flex gap-2 bg-card border border-border rounded-lg p-2 hover:border-foreground/20 transition-colors cursor-pointer"
      onClick={() => onOpenPost?.(post)}
    >
      <VoteButton targetId={post.name} score={post.score} initialLikes={post.likes} />

      <div className="flex-1 min-w-0">
        {/* Meta line */}
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1 flex-wrap">
          <button
            onClick={(e) => { e.stopPropagation(); onOpenSubreddit?.(post.subreddit); }}
            className="flex items-center gap-1 font-semibold hover:underline text-foreground"
          >
            <SubredditAvatar name={post.subreddit} size={18} />
            r/{post.subreddit}
          </button>
          <span>•</span>
          <span>
            Posted by{' '}
            <button
              onClick={(e) => { e.stopPropagation(); onOpenUser?.(post.author); }}
              className="hover:underline"
            >
              u/{post.author}
            </button>{' '}
            {timeAgo(post.created_at)}
          </span>
          {post.is_sticky && <span className="bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded text-[10px] font-semibold">STICKIED</span>}
          {post.is_locked && <span className="bg-yellow-500/20 text-yellow-700 px-1.5 py-0.5 rounded text-[10px] font-semibold">LOCKED</span>}
          {post.over18 && <span className="bg-red-500/20 text-red-600 px-1.5 py-0.5 rounded text-[10px] font-semibold">NSFW</span>}
          {post.is_spoiler && <span className="bg-foreground/20 text-foreground px-1.5 py-0.5 rounded text-[10px] font-semibold">SPOILER</span>}
        </div>

        {/* Title */}
        <h2 className="text-base font-semibold leading-tight text-foreground mb-2">
          {post.title}
        </h2>

        {/* Flair */}
        {post.flair_text && (
          <span
            className="inline-block text-[10px] font-semibold px-1.5 py-0.5 rounded mr-2 mb-1"
            style={{
              backgroundColor: post.flair_color ? `${post.flair_color}30` : 'rgba(0,121,211,0.1)',
              color: post.flair_color || '#0079D3',
            }}
          >
            {post.flair_text}
          </span>
        )}

        {/* Body preview / link card */}
        {isSelf && post.body && (
          <div className="text-sm text-muted-foreground line-clamp-3 mb-2 overflow-hidden">
            <Markdown content={post.body} />
          </div>
        )}
        {isLink && (
          <div className="mb-2">
            <a
              href={post.url!}
              target="_blank"
              rel="noopener noreferrer"
              onClick={(e) => e.stopPropagation()}
              className="block bg-accent/30 rounded-lg p-3 hover:bg-accent/50 transition-colors"
            >
              {post.thumbnail && !imageError && post.thumbnail !== 'self' && post.thumbnail !== 'default' && post.thumbnail !== 'nsfw' && (
                <img
                  src={post.thumbnail}
                  alt=""
                  className="mb-2 rounded max-h-48 object-cover w-full"
                  onError={() => setImageError(true)}
                />
              )}
              <div className="flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
                <Link2 className="w-4 h-4 shrink-0" />
                <span className="truncate">{post.url}</span>
                {domain && <span className="text-muted-foreground text-xs">({domain})</span>}
              </div>
            </a>
          </div>
        )}

        {/* Action bar */}
        <div className="flex items-center gap-1 text-xs text-muted-foreground -ml-1">
          <button
            onClick={(e) => { e.stopPropagation(); onOpenPost?.(post); }}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent transition-colors font-semibold"
          >
            <MessageSquare className="w-4 h-4" />
            {formatCount(post.num_comments)} Comments
          </button>
          <button
            onClick={async (e) => {
              e.stopPropagation();
              if (!user) { alert('Please log in to share.'); return; }
              try {
                if (navigator.share) {
                  await navigator.share({ title: post.title, url: post.url || `${window.location.origin}/#/post/${post.id}` });
                } else {
                  await navigator.clipboard?.writeText(post.url || `${window.location.origin}/#/post/${post.id}`);
                  alert('Link copied');
                }
              } catch {}
            }}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent transition-colors"
          >
            <Share2 className="w-4 h-4" />
            Share
          </button>
          <button
            onClick={async (e) => {
              e.stopPropagation();
              if (!user) { alert('Please log in to save posts.'); return; }
              try {
                if (saved) { await api.unsavePost(post.id); setSaved(false); }
                else { await api.savePost(post.id); setSaved(true); }
              } catch (err) { alert((err as Error).message); }
            }}
            className={cn(
              'flex items-center gap-1 px-2 py-1 rounded hover:bg-accent transition-colors',
              saved && 'text-orange-600 dark:text-orange-400 font-semibold'
            )}
          >
            <Bookmark className={cn('w-4 h-4', saved && 'fill-current')} />
            Save
          </button>
          <button
            onClick={async (e) => {
              e.stopPropagation();
              if (!user) { alert('Please log in to hide posts.'); return; }
              try {
                await api.hidePost(post.id);
                setHidden(true);
              } catch (err) { alert((err as Error).message); }
            }}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent transition-colors"
          >
            <EyeOff className="w-4 h-4" />
            Hide
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              const reason = prompt('Report this post. Reason:');
              if (!reason) return;
              api.createReport({
                target_id: post.id,
                target_type: 'post',
                reason,
                subreddit: post.subreddit,
              }).then(() => alert('Reported.'))
                .catch((err) => alert(err.message));
            }}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent transition-colors"
          >
            <Flag className="w-4 h-4" />
            Report
          </button>
          <button
            onClick={(e) => e.stopPropagation()}
            className="px-2 py-1 rounded hover:bg-accent transition-colors"
          >
            <MoreHorizontal className="w-4 h-4" />
          </button>
        </div>
      </div>
    </article>
  );
}
