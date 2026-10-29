'use client';
import { useAuth } from '@/lib/auth-context';

import { useState } from 'react';
import { Reply, MoreHorizontal, Share2, Flag, Award, ChevronDown, Pencil, Trash2 } from 'lucide-react';
import { Comment, api } from '@/lib/api';
import { timeAgo } from '@/lib/format';
import { VoteButton } from './vote-button';
import { UserAvatar } from './subreddit-avatar';
import { Markdown } from './markdown';
import { cn } from '@/lib/utils';

interface CommentTreeProps {
  comment: Comment;
  postId: string;
  depth: number;
  onOpenUser: (name: string) => void;
}

export function CommentTree({ comment, postId, depth, onOpenUser }: CommentTreeProps) {
  const { user } = useAuth();
  const [collapsed, setCollapsed] = useState(false);
  const [replying, setReplying] = useState(false);
  const [replyText, setReplyText] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(comment.body);

  const handleReply = async () => {
    if (!user) {
      alert('Please log in to reply.');
      return;
    }
    if (!replyText.trim()) return;
    setSubmitting(true);
    try {
      await api.createComment(postId, replyText, comment.id);
      setReplyText('');
      setReplying(false);
      window.dispatchEvent(new CustomEvent('comment-added', { detail: { postId } }));
    } catch (e) {
      alert((e as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  if (collapsed) {
    return (
      <div className="py-1">
        <button
          onClick={() => setCollapsed(false)}
          className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
        >
          <ChevronDown className="w-3 h-3 rotate-90" />
          <UserAvatar name={comment.author === '[deleted]' ? '?' : comment.author} size={16} />
          <span className="font-semibold">u/{comment.author}</span>
          <span>•</span>
          <span>{comment.score} points</span>
          <ChevronDown className="w-3 h-3 -rotate-90" />
        </button>
      </div>
    );
  }

  return (
    <div className={cn('flex gap-2', depth > 0 && 'ml-3 border-l-2 border-border pl-3')}>
      <UserAvatar name={comment.author === '[deleted]' ? '?' : comment.author} size={24} className="mt-0.5" />

      <div className="flex-1 min-w-0">
        {/* Author meta */}
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <button
            onClick={() => comment.author !== '[deleted]' && onOpenUser(comment.author)}
            className={cn(
              'font-semibold hover:underline',
              comment.is_op ? 'text-blue-600 dark:text-blue-400' : 'text-foreground'
            )}
          >
            u/{comment.author}
          </button>
          {comment.is_op && <span className="bg-blue-500/20 text-blue-600 dark:text-blue-400 px-1.5 py-0.5 rounded text-[10px] font-semibold">OP</span>}
          <span>•</span>
          <span>{timeAgo(comment.created_at)}</span>
          {comment.is_sticky && <span className="bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded text-[10px] font-semibold">STICKIED</span>}
          {comment.is_locked && <span className="bg-yellow-500/20 text-yellow-700 px-1.5 py-0.5 rounded text-[10px] font-semibold">LOCKED</span>}
        </div>

        {/* Body */}
        {editing ? (
          <div className="my-1">
            <textarea
              value={editText}
              onChange={(e) => setEditText(e.target.value)}
              rows={3}
              className="w-full bg-accent/20 border border-border rounded p-2 text-sm outline-none focus:border-orange-500 resize-y"
            />
            <div className="flex justify-end gap-2 mt-1">
              <button
                onClick={() => { setEditing(false); setEditText(comment.body); }}
                className="px-3 py-1 rounded-full text-xs font-semibold border border-border hover:bg-accent"
              >
                Cancel
              </button>
              <button
                onClick={async () => {
                  try {
                    await api.editComment(comment.id, editText);
                    setEditing(false);
                    window.dispatchEvent(new CustomEvent('comment-added', { detail: { postId } }));
                  } catch (err) { alert((err as Error).message); }
                }}
                className="px-3 py-1 rounded-full text-xs font-semibold bg-orange-500 hover:bg-orange-600 text-white"
              >
                Save
              </button>
            </div>
          </div>
        ) : (
          <div className="text-sm text-foreground/90 leading-relaxed my-1">
            <Markdown content={comment.body} />
          </div>
        )}

        {/* Actions */}
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <VoteButton targetId={comment.name} score={comment.score} initialLikes={comment.likes} variant="comment" />
          <button
            onClick={() => setReplying((v) => !v)}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent font-semibold"
          >
            <Reply className="w-3 h-3" /> Reply
          </button>
          <button className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent">
            <Share2 className="w-3 h-3" /> Share
          </button>
          <button className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent">
            <Award className="w-3 h-3" /> Award
          </button>
          <button
            onClick={() => {
              const reason = prompt('Report this comment. Reason:');
              if (!reason) return;
              api.createReport({
                target_id: comment.id,
                target_type: 'comment',
                reason,
                subreddit: comment.subreddit,
              }).then(() => alert('Reported.'))
                .catch((err) => alert(err.message));
            }}
            className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
          >
            <Flag className="w-3 h-3" /> Report
          </button>
          {user && user.id === comment.author_id && comment.author !== '[deleted]' && (
            <>
              <button
                onClick={() => setEditing(true)}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent"
              >
                <Pencil className="w-3 h-3" /> Edit
              </button>
              <button
                onClick={() => {
                  if (!confirm('Delete this comment?')) return;
                  api.deleteComment(comment.id)
                    .then(() => window.dispatchEvent(new CustomEvent('comment-added', { detail: { postId } })))
                    .catch((err) => alert(err.message));
                }}
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent text-red-500"
              >
                <Trash2 className="w-3 h-3" /> Delete
              </button>
            </>
          )}
          <button
            onClick={() => setCollapsed(true)}
            className="px-2 py-1 rounded hover:bg-accent"
          >
            <MoreHorizontal className="w-3 h-3" />
          </button>
        </div>

        {/* Reply box */}
        {replying && (
          <div className="mt-2">
            <p className="text-xs text-muted-foreground mb-1">
              Replying as <span className="font-semibold text-foreground">u/{user?.username || 'guest'}</span>
            </p>
            <textarea
              value={replyText}
              onChange={(e) => setReplyText(e.target.value)}
              placeholder="What are your thoughts?"
              rows={3}
              className="w-full bg-accent/20 border border-border rounded-lg p-2 text-sm outline-none focus:border-orange-500 resize-y"
              disabled={submitting}
            />
            <div className="flex justify-end gap-2 mt-1">
              <button
                onClick={() => { setReplying(false); setReplyText(''); }}
                className="px-3 py-1 rounded-full text-xs font-semibold border border-border hover:bg-accent"
                disabled={submitting}
              >
                Cancel
              </button>
              <button
                onClick={handleReply}
                disabled={submitting || !replyText.trim()}
                className="px-3 py-1 rounded-full text-xs font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50"
              >
                {submitting ? 'Replying…' : 'Reply'}
              </button>
            </div>
          </div>
        )}

        {/* Nested replies */}
        {comment.replies.length > 0 && (
          <div className="mt-2 space-y-2">
            {comment.replies.map((r) => (
              <CommentTree
                key={r.id}
                comment={r}
                postId={postId}
                depth={depth + 1}
                onOpenUser={onOpenUser}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
