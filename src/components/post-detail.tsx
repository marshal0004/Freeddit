'use client';
import { useAuth } from '@/lib/auth-context';

import { useState, useRef } from 'react';
import { MessageSquare, Share2, Bookmark, Flag, ArrowLeft, Reply, EyeOff, Pencil, Trash2 } from 'lucide-react';
import { Post, Comment, CommentSort, api } from '@/lib/api';
import { timeAgo, formatCount } from '@/lib/format';
import { VoteButton } from './vote-button';
import { SubredditAvatar, UserAvatar } from './subreddit-avatar';
import { Markdown } from './markdown';
import { SubredditSidebar } from './subreddit-sidebar';
import { cn } from '@/lib/utils';
import { CommentTree } from './comment-tree';

interface PostDetailProps {
  post: Post;
  sort: CommentSort;
  onSortChange: (s: CommentSort) => void;
  comments: Comment[];
  isLoadingComments: boolean;
  onOpenSubreddit: (name: string) => void;
  onOpenUser: (name: string) => void;
  onBack: () => void;
}

export function PostDetail({
  post, sort, onSortChange, comments, isLoadingComments,
  onOpenSubreddit, onOpenUser, onBack,
}: PostDetailProps) {
  const { user } = useAuth();
  const [newComment, setNewComment] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [saved, setSaved] = useState(false);
  const [hidden, setHidden] = useState(false);
  const [editingPost, setEditingPost] = useState(false);
  const editTextareaRef = useRef<HTMLTextAreaElement>(null);

  const submitComment = async () => {
    if (!user) {
      alert('Please log in to comment.');
      return;
    }
    if (!newComment.trim()) return;
    setSubmitting(true);
    try {
      await api.createComment(post.id, newComment);
      setNewComment('');
      // Refresh handled by parent via re-fetch (we just clear and let React Query refetch)
      window.dispatchEvent(new CustomEvent('comment-added', { detail: { postId: post.id } }));
    } catch (e) {
      alert((e as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="flex max-w-7xl mx-auto gap-3">
      <div className="flex-1 min-w-0 max-w-3xl">
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground mb-3"
      >
        <ArrowBack /> Back
      </button>

      <article className="bg-card border border-border rounded-lg overflow-hidden">
        <div className="flex gap-2 p-3">
          <VoteButton targetId={post.name} score={post.score} initialLikes={post.likes} />

          <div className="flex-1 min-w-0">
            {/* Meta */}
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5 flex-wrap">
              <button
                onClick={() => onOpenSubreddit(post.subreddit)}
                className="flex items-center gap-1 font-semibold hover:underline text-foreground"
              >
                <SubredditAvatar name={post.subreddit} size={20} />
                r/{post.subreddit}
              </button>
              <span>•</span>
              <span>
                Posted by{' '}
                <button
                  onClick={() => onOpenUser(post.author)}
                  className="hover:underline"
                >u/{post.author}</button>{' '}
                {timeAgo(post.created_at)}
              </span>
              {post.is_sticky && <span className="bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded text-[10px] font-semibold">STICKIED</span>}
              {post.is_locked && <span className="bg-yellow-500/20 text-yellow-700 px-1.5 py-0.5 rounded text-[10px] font-semibold">LOCKED</span>}
            </div>

            {/* Title */}
            <h1 className="text-xl font-bold leading-tight mb-2">{post.title}</h1>

            {/* Flair */}
            {post.flair_text && (
              <span
                className="inline-block text-xs font-semibold px-2 py-0.5 rounded mb-2"
                style={{
                  backgroundColor: post.flair_color ? `${post.flair_color}30` : 'rgba(0,121,211,0.1)',
                  color: post.flair_color || '#0079D3',
                }}
              >
                {post.flair_text}
              </span>
            )}

            {/* Body — rendered as markdown */}
            {post.is_self && post.body && (
              <div className="text-sm text-foreground/90 mb-3 leading-relaxed">
                <Markdown content={post.body} />
              </div>
            )}
            {post.kind === 'link' && post.url && (
              <a
                href={post.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 dark:text-blue-400 hover:underline break-all text-sm mb-3 block"
              >
                {post.url}
              </a>
            )}

            {/* Action bar */}
            <div className="flex items-center gap-2 text-xs text-muted-foreground flex-wrap">
              <span className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent font-semibold">
                <MessageSquare className="w-4 h-4" />
                {formatCount(post.num_comments)} Comments
              </span>
              <button
                onClick={async (e) => {
                  e.stopPropagation();
                  try {
                    if (navigator.share) {
                      await navigator.share({ title: post.title, url: post.url || `${window.location.origin}/#/post/${post.id}` });
                    } else {
                      await navigator.clipboard?.writeText(post.url || `${window.location.origin}/#/post/${post.id}`);
                      alert('Link copied to clipboard');
                    }
                  } catch (e) { /* user cancelled */ }
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
              >
                <Share2 className="w-4 h-4" /> Share
              </button>
              <button
                onClick={async (e) => {
                  e.stopPropagation();
                  if (!user) { alert('Please log in to save posts.'); return; }
                  try {
                    if (saved) {
                      await api.unsavePost(post.id);
                      setSaved(false);
                    } else {
                      await api.savePost(post.id);
                      setSaved(true);
                    }
                  } catch (err) { alert((err as Error).message); }
                }}
                className={cn(
                  'flex items-center gap-1 px-2 py-1 rounded hover:bg-accent',
                  saved && 'text-orange-600 dark:text-orange-400 font-semibold'
                )}
              >
                <Bookmark className={cn('w-4 h-4', saved && 'fill-current')} /> {saved ? 'Saved' : 'Save'}
              </button>
              <button
                onClick={async (e) => {
                  e.stopPropagation();
                  if (!user) { alert('Please log in to hide posts.'); return; }
                  try {
                    if (hidden) {
                      await api.unhidePost(post.id);
                      setHidden(false);
                    } else {
                      await api.hidePost(post.id);
                      setHidden(true);
                    }
                  } catch (err) { alert((err as Error).message); }
                }}
                className={cn(
                  'flex items-center gap-1 px-2 py-1 rounded hover:bg-accent',
                  hidden && 'text-blue-600 dark:text-blue-400 font-semibold'
                )}
              >
                <EyeOff className="w-4 h-4" /> {hidden ? 'Unhide' : 'Hide'}
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
                  }).then(() => alert('Reported. Thanks for keeping freeddit safe.'))
                    .catch((err) => alert(err.message));
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
              >
                <Flag className="w-4 h-4" /> Report
              </button>
              {user && user.id === post.author_id && (
                <>
                  <button
                    onClick={(e) => { e.stopPropagation(); setEditingPost(true); }}
                    className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
                  >
                    <Pencil className="w-4 h-4" /> Edit
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (!confirm('Delete this post? This cannot be undone.')) return;
                      api.deletePost(post.id).then(() => {
                        alert('Post deleted');
                        window.location.hash = '';
                      }).catch((err) => alert(err.message));
                    }}
                    className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent text-red-500"
                  >
                    <Trash2 className="w-4 h-4" /> Delete
                  </button>
                </>
              )}
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  // Trigger create-post modal in crosspost mode via a custom event
                  window.dispatchEvent(new CustomEvent('crosspost-trigger', { detail: { postId: post.id, title: post.title } }));
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
                title="Crosspost to another subreddit"
              >
                <Share2 className="w-4 h-4 rotate-90" /> Crosspost
              </button>
            </div>

            {/* Inline edit form */}
            {editingPost && user && user.id === post.author_id && (
              <div className="mt-3 bg-accent/20 border border-border rounded-lg p-3">
                <textarea
                  defaultValue={post.body || ''}
                  rows={6}
                  ref={(el) => { editTextareaRef.current = el; }}
                  className="w-full bg-background border border-border rounded p-2 text-sm outline-none focus:border-orange-500"
                />
                <div className="flex justify-end gap-2 mt-2">
                  <button
                    onClick={() => setEditingPost(false)}
                    className="px-3 py-1 rounded-full text-xs font-semibold border border-border hover:bg-accent"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={async () => {
                      if (!editTextareaRef.current) return;
                      const newBody = editTextareaRef.current.value;
                      try {
                        await api.editPost(post.id, { body: newBody });
                        setEditingPost(false);
                        window.dispatchEvent(new CustomEvent('post-edited', { detail: { postId: post.id } }));
                      } catch (err) { alert((err as Error).message); }
                    }}
                    className="px-3 py-1 rounded-full text-xs font-semibold bg-orange-500 hover:bg-orange-600 text-white"
                  >
                    Save
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Comment box */}
        <div className="border-t border-border p-3">
          <p className="text-xs text-muted-foreground mb-2">
            Comment as <span className="font-semibold text-foreground">{user ? `u/${user.username}` : 'a guest'}</span>
          </p>
          {!user ? (
            <p className="text-sm text-muted-foreground italic">Log in or sign up to leave a comment.</p>
          ) : (
            <>
              <textarea
                value={newComment}
                onChange={(e) => setNewComment(e.target.value)}
                placeholder="What are your thoughts?"
                rows={4}
                className="w-full bg-accent/20 border border-border rounded-lg p-3 text-sm outline-none focus:border-orange-500 resize-y min-h-24"
                disabled={submitting || post.is_locked}
              />
              <div className="flex justify-end gap-2 mt-2">
                <button
                  onClick={() => setNewComment('')}
                  className="px-4 py-1.5 rounded-full text-sm font-semibold border border-border hover:bg-accent"
                  disabled={submitting}
                >
                  Cancel
                </button>
                <button
                  onClick={submitComment}
                  disabled={submitting || !newComment.trim() || post.is_locked}
                  className="px-4 py-1.5 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {submitting ? 'Commenting…' : 'Comment'}
                </button>
              </div>
            </>
          )}
        </div>

        {/* Sort tabs */}
        <div className="border-t border-border p-3 flex items-center gap-2 text-xs">
          <span className="font-semibold text-muted-foreground">Sort by:</span>
          {(['best', 'top', 'new', 'controversial', 'old', 'qa'] as CommentSort[]).map((s) => (
            <button
              key={s}
              onClick={() => onSortChange(s)}
              className={cn(
                'px-2 py-1 rounded font-semibold capitalize',
                sort === s ? 'bg-orange-500/15 text-orange-600 dark:text-orange-400' : 'hover:bg-accent text-muted-foreground'
              )}
            >
              {s}
            </button>
          ))}
        </div>

        {/* Comments */}
        <div className="p-3 space-y-2">
          {isLoadingComments ? (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-20 bg-accent/30 animate-pulse rounded" />
              ))}
            </div>
          ) : comments.length === 0 ? (
            <p className="text-sm text-muted-foreground italic py-4 text-center">
              No comments yet. Be the first to comment!
            </p>
          ) : (
            comments.map((c) => (
              <CommentTree
                key={c.id}
                comment={c}
                postId={post.id}
                depth={0}
                onOpenUser={onOpenUser}
              />
            ))
          )}
        </div>
      </article>
      </div>
      <SubredditSidebar subredditName={post.subreddit} onOpenUser={onOpenUser} />
    </div>
  );
}

function ArrowBack() {
  return <ArrowLeft className="w-4 h-4" />;
}
