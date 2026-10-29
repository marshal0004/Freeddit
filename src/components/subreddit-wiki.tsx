'use client';

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { BookOpen, Pencil, Plus, Trash2, X, Save, ChevronRight } from 'lucide-react';
import { api } from '@/lib/api';
import { Markdown } from './markdown';
import { timeAgo } from '@/lib/format';
import { cn } from '@/lib/utils';

interface SubredditWikiProps {
  subredditName: string;
  isMod: boolean;
}

export function SubredditWiki({ subredditName, isMod }: SubredditWikiProps) {
  const qc = useQueryClient();
  const [openSlug, setOpenSlug] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [editSlug, setEditSlug] = useState('');
  const [editTitle, setEditTitle] = useState('');
  const [editContent, setEditContent] = useState('');
  const [showNewPageForm, setShowNewPageForm] = useState(false);

  const { data: pagesList } = useQuery({
    queryKey: ['wiki-pages', subredditName],
    queryFn: () => api.listWikiPages(subredditName),
  });

  const { data: openPage } = useQuery({
    queryKey: ['wiki-page', subredditName, openSlug],
    queryFn: () => api.getWikiPage(subredditName, openSlug!),
    enabled: !!openSlug,
  });

  const upsertMut = useMutation({
    mutationFn: () => api.upsertWikiPage(subredditName, { slug: editSlug, title: editTitle, content: editContent }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['wiki-pages', subredditName] });
      qc.invalidateQueries({ queryKey: ['wiki-page', subredditName, editSlug] });
      setEditing(false);
      setOpenSlug(editSlug);
      setShowNewPageForm(false);
    },
  });

  const deleteMut = useMutation({
    mutationFn: (slug: string) => api.deleteWikiPage(subredditName, slug),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['wiki-pages', subredditName] });
      setOpenSlug(null);
    },
  });

  return (
    <div className="flex gap-4">
      {/* Pages list */}
      <aside className="w-56 shrink-0">
        <div className="bg-card border border-border rounded-lg overflow-hidden">
          <div className="bg-accent/40 px-3 py-2 font-semibold text-sm flex items-center gap-2">
            <BookOpen className="w-4 h-4" /> Wiki
          </div>
          <ul className="divide-y divide-border text-sm">
            <li>
              <button
                onClick={() => { setOpenSlug(null); setShowNewPageForm(false); }}
                className={cn('w-full text-left px-3 py-2 hover:bg-accent', !openSlug && !showNewPageForm && 'bg-accent/50 font-semibold')}
              >
                Index
              </button>
            </li>
            {pagesList?.pages.map(p => (
              <li key={p.slug}>
                <button
                  onClick={() => { setOpenSlug(p.slug); setShowNewPageForm(false); }}
                  className={cn('w-full text-left px-3 py-2 hover:bg-accent flex items-center gap-1', openSlug === p.slug && 'bg-accent/50 font-semibold')}
                >
                  <ChevronRight className="w-3 h-3 opacity-50" />
                  {p.slug}
                </button>
              </li>
            ))}
          </ul>
          {isMod && (
            <button
              onClick={() => { setShowNewPageForm(true); setOpenSlug(null); setEditSlug(''); setEditTitle(''); setEditContent(''); setEditing(true); }}
              className="w-full text-xs font-semibold text-orange-600 dark:text-orange-400 hover:bg-accent px-3 py-2 border-t border-border flex items-center gap-1"
            >
              <Plus className="w-3 h-3" /> New page
            </button>
          )}
        </div>
      </aside>

      {/* Page content */}
      <div className="flex-1 min-w-0 bg-card border border-border rounded-lg p-4">
        {showNewPageForm && editing ? (
          <NewPageForm
            editSlug={editSlug} setEditSlug={setEditSlug}
            editTitle={editTitle} setEditTitle={setEditTitle}
            editContent={editContent} setEditContent={setEditContent}
            onSave={() => upsertMut.mutate()}
            onCancel={() => { setShowNewPageForm(false); setEditing(false); }}
            busy={upsertMut.isLoading}
          />
        ) : openPage && !editing ? (
          <div>
            <div className="flex items-center justify-between mb-3">
              <div>
                <h1 className="text-2xl font-bold">{openPage.title}</h1>
                <p className="text-xs text-muted-foreground mt-1">r/{subredditName}/wiki/{openPage.slug} · updated {timeAgo(openPage.updated_at)}</p>
              </div>
              {isMod && (
                <div className="flex gap-2">
                  <button
                    onClick={() => { setEditing(true); setEditSlug(openPage.slug); setEditTitle(openPage.title); setEditContent(openPage.content); setShowNewPageForm(false); }}
                    className="px-3 py-1.5 rounded-full text-xs font-semibold border border-border hover:bg-accent flex items-center gap-1"
                  >
                    <Pencil className="w-3 h-3" /> Edit
                  </button>
                  <button
                    onClick={() => { if (confirm('Delete this wiki page?')) deleteMut.mutate(openPage.slug); }}
                    className="px-3 py-1.5 rounded-full text-xs font-semibold border border-border text-red-500 hover:bg-red-500/10 flex items-center gap-1"
                  >
                    <Trash2 className="w-3 h-3" /> Delete
                  </button>
                </div>
              )}
            </div>
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <Markdown content={openPage.content} />
            </div>
          </div>
        ) : openPage && editing ? (
          <EditForm
            editSlug={editSlug} setEditSlug={setEditSlug}
            editTitle={editTitle} setEditTitle={setEditTitle}
            editContent={editContent} setEditContent={setEditContent}
            onSave={() => upsertMut.mutate()}
            onCancel={() => setEditing(false)}
            busy={upsertMut.isLoading}
          />
        ) : (
          <div>
            <h1 className="text-2xl font-bold mb-2">r/{subredditName} wiki</h1>
            <p className="text-sm text-muted-foreground mb-4">
              Welcome to the r/{subredditName} wiki. Choose a page from the left to read.
            </p>
            {pagesList && pagesList.pages.length === 0 && (
              <p className="text-xs text-muted-foreground italic">No wiki pages yet. {isMod && 'Mods can create one with the "New page" button above.'}</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function NewPageForm(props: EditFormProps) {
  return (
    <div>
      <h2 className="text-xl font-bold mb-3">New wiki page</h2>
      <EditForm {...props} showSlug />
    </div>
  );
}

interface EditFormProps {
  editSlug: string; setEditSlug: (v: string) => void;
  editTitle: string; setEditTitle: (v: string) => void;
  editContent: string; setEditContent: (v: string) => void;
  onSave: () => void; onCancel: () => void;
  busy: boolean;
  showSlug?: boolean;
}

function EditForm({ editSlug, setEditSlug, editTitle, setEditTitle, editContent, setEditContent, onSave, onCancel, busy, showSlug }: EditFormProps) {
  return (
    <div>
      {showSlug && (
        <div className="mb-3">
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Slug (URL)</label>
          <input value={editSlug} onChange={(e) => setEditSlug(e.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, '-'))} placeholder="e.g. rules, faq" maxLength={64} className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
        </div>
      )}
      <div className="mb-3">
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Title</label>
        <input value={editTitle} onChange={(e) => setEditTitle(e.target.value)} placeholder="Page title" maxLength={200} className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500" />
      </div>
      <div className="mb-3">
        <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Content (markdown)</label>
        <textarea value={editContent} onChange={(e) => setEditContent(e.target.value)} rows={12} placeholder="# Markdown supported…" className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm font-mono outline-none focus:border-orange-500 resize-y" />
      </div>
      <div className="flex justify-end gap-2">
        <button onClick={onCancel} className="px-4 py-1.5 rounded-full text-sm font-semibold border border-border hover:bg-accent">Cancel</button>
        <button onClick={onSave} disabled={busy || !editTitle || !editContent || (showSlug && !editSlug)} className="px-4 py-1.5 rounded-full text-sm font-semibold bg-orange-500 hover:bg-orange-600 text-white disabled:opacity-50 flex items-center gap-2">
          <Save className="w-4 h-4" /> {busy ? 'Saving…' : 'Save page'}
        </button>
      </div>
    </div>
  );
}
