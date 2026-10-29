'use client';
import { useAuth } from '@/lib/auth-context';

import { useState, useRef } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { X, Link2, FileText, Image as ImageIcon, Loader2 } from 'lucide-react';
import { api } from '@/lib/api';
import { cn } from '@/lib/utils';

interface CreatePostModalProps {
  onClose: () => void;
  defaultSubreddit?: string;
  onCreated: (postPath: string) => void;
  crosspostParentId?: string;
  crosspostParentTitle?: string;
}

type TabType = 'post' | 'link' | 'image';

export function CreatePostModal({ onClose, defaultSubreddit, onCreated, crosspostParentId, crosspostParentTitle }: CreatePostModalProps) {
  const { user } = useAuth();
  const qc = useQueryClient();
  const [tab, setTab] = useState<TabType>('post');
  const [subreddit, setSubreddit] = useState(defaultSubreddit || '');
  const [title, setTitle] = useState(crosspostParentId && crosspostParentTitle ? crosspostParentTitle : '');
  const [body, setBody] = useState('');
  const [url, setUrl] = useState('');
  const [mediaUrl, setMediaUrl] = useState<string | null>(null);
  const [mediaKind, setMediaKind] = useState<string | null>(null);
  const [isNsfw, setIsNsfw] = useState(false);
  const [isSpoiler, setIsSpoiler] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const { data: subsList } = useQuery({
    queryKey: ['my-subreddits'],
    queryFn: () => api.listSubreddits(100),
  });

  const createMut = useMutation({
    mutationFn: () => {
      const kind = tab === 'post' ? 'self' : tab === 'image' ? 'image' : 'link';
      return api.createPost({
        subreddit,
        title,
        body: tab === 'post' ? body : undefined,
        url: tab === 'link' ? url : undefined,
        kind,
        is_nsfw: isNsfw,
        is_spoiler: isSpoiler,
        media_url: tab === 'image' && mediaUrl ? mediaUrl : undefined,
        media_kind: tab === 'image' && mediaKind ? mediaKind : undefined,
        crosspost_parent_id: crosspostParentId,
      });
    },
    onSuccess: (res) => {
      qc.invalidateQueries({ queryKey: ['posts'] });
      qc.invalidateQueries({ queryKey: ['subreddit-posts'] });
      onCreated(`/post/${res.data.id}`);
      onClose();
    },
    onError: (e: Error) => setError(e.message),
  });

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setUploading(true);
    setError(null);
    try {
      const res = await api.uploadMedia(file);
      setMediaUrl(res.url);
      setMediaKind(res.kind);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setUploading(false);
    }
  };

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!subreddit.trim()) return setError('Please choose a subreddit.');
    if (!title.trim()) return setError('Title is required.');
    if (tab === 'link' && !url.trim()) return setError('URL is required for link posts.');
    if (tab === 'image' && !mediaUrl) return setError('Please upload an image.');
    createMut.mutate();
  };

  if (!user) {
    return (
      <div className="fixed inset-0 z-[100] bg-black/60 flex items-center justify-center p-4" onClick={onClose}>
        <div className="bg-card border border-border rounded-xl p-6 max-w-md text-center" onClick={(e) => e.stopPropagation()}>
          <p>Please log in to create a post.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-[100] bg-black/60 flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-card border border-border rounded-xl w-full max-w-2xl p-6 relative max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          onClick={onClose}
          className="absolute top-3 right-3 p-1.5 rounded-full hover:bg-accent"
          aria-label="Close"
        >
          <X className="w-4 h-4" />
        </button>

        <h2 className="text-xl font-bold mb-1">{crosspostParentId ? 'Crosspost' : 'Create a post'}</h2>
        {crosspostParentId && crosspostParentTitle && (
          <p className="text-xs text-muted-foreground mb-3">
            Crossposting from <span className="font-semibold">{crosspostParentTitle}</span>
          </p>
        )}

        {error && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-600 dark:text-red-400 text-sm rounded-lg p-3 mb-4">
            {error}
          </div>
        )}

        {/* Subreddit selector */}
        <div className="mb-4">
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Choose a community</label>
          <select
            value={subreddit}
            onChange={(e) => setSubreddit(e.target.value)}
            className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
          >
            <option value="">Select a subreddit</option>
            {(subsList?.data.children || []).map(({ data: s }) => (
              <option key={s.id} value={s.display_name}>r/{s.display_name}</option>
            ))}
          </select>
        </div>

        {/* Tab switch */}
        <div className="flex gap-1 bg-accent/30 p-1 rounded-lg mb-4">
          <button
            type="button"
            onClick={() => setTab('post')}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 py-1.5 rounded-md text-sm font-semibold',
              tab === 'post' ? 'bg-card text-foreground' : 'text-muted-foreground'
            )}
          >
            <FileText className="w-4 h-4" /> Post
          </button>
          <button
            type="button"
            onClick={() => setTab('image')}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 py-1.5 rounded-md text-sm font-semibold',
              tab === 'image' ? 'bg-card text-foreground' : 'text-muted-foreground'
            )}
          >
            <ImageIcon className="w-4 h-4" /> Image
          </button>
          <button
            type="button"
            onClick={() => setTab('link')}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 py-1.5 rounded-md text-sm font-semibold',
              tab === 'link' ? 'bg-card text-foreground' : 'text-muted-foreground'
            )}
          >
            <Link2 className="w-4 h-4" /> Link
          </button>
        </div>

        <form onSubmit={submit} className="space-y-3">
          <div>
            <input
              type="text"
              required
              maxLength={300}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
              placeholder="Title"
            />
            <p className="text-xs text-muted-foreground mt-1 text-right">{title.length}/300</p>
          </div>

          {tab === 'post' && (
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              rows={8}
              maxLength={40000}
              className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 resize-y font-mono"
              placeholder="Text (markdown supported)"
            />
          )}

          {tab === 'image' && (
            <div className="space-y-3">
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*,video/*"
                onChange={handleFileSelect}
                className="hidden"
              />
              {mediaUrl ? (
                <div className="border border-border rounded-lg p-3 bg-accent/20">
                  {mediaKind === 'image' ? (
                    <img src={mediaUrl} alt="upload preview" className="max-h-64 mx-auto rounded" />
                  ) : mediaKind === 'video' ? (
                    <video src={mediaUrl} controls className="max-h-64 mx-auto rounded w-full" />
                  ) : null}
                  <div className="flex justify-between items-center mt-2">
                    <p className="text-xs text-muted-foreground truncate">{mediaUrl}</p>
                    <button
                      type="button"
                      onClick={() => { setMediaUrl(null); setMediaKind(null); if (fileInputRef.current) fileInputRef.current.value = ''; }}
                      className="text-xs text-red-500 hover:underline"
                    >
                      Remove
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => fileInputRef.current?.click()}
                  disabled={uploading}
                  className="w-full border-2 border-dashed border-border rounded-lg p-8 text-center hover:bg-accent/30 transition-colors disabled:opacity-50"
                >
                  {uploading ? (
                    <>
                      <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2" />
                      <p className="text-sm">Uploading…</p>
                    </>
                  ) : (
                    <>
                      <ImageIcon className="w-8 h-8 mx-auto mb-2 text-muted-foreground" />
                      <p className="text-sm font-semibold">Click to upload image or video</p>
                      <p className="text-xs text-muted-foreground mt-1">Max 50 MB · Images & videos</p>
                    </>
                  )}
                </button>
              )}
            </div>
          )}

          {tab === 'link' && (
            <input
              type="url"
              required
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              className="w-full bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
              placeholder="URL"
            />
          )}

          <div className="flex gap-4">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={isNsfw}
                onChange={(e) => setIsNsfw(e.target.checked)}
                className="rounded"
              />
              NSFW
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={isSpoiler}
                onChange={(e) => setIsSpoiler(e.target.checked)}
                className="rounded"
              />
              Spoiler
            </label>
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-1.5 rounded-full text-sm font-semibold border border-border hover:bg-accent"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={createMut.isLoading || uploading}
              className="px-4 py-1.5 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50"
            >
              {createMut.isLoading ? 'Posting…' : 'Post'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
