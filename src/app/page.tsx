'use client';
import { useAuth } from '@/lib/auth-context';

import { useEffect, useState, useCallback } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Header } from '@/components/header';
import { Sidebar } from '@/components/sidebar';
import { SortTabs } from '@/components/sort-tabs';
import { PostCard } from '@/components/post-card';
import { PostDetail } from '@/components/post-detail';
import { SubredditHeader } from '@/components/subreddit-header';
import { UserPage } from '@/components/user-page';
import { SearchPage } from '@/components/search-page';
import { AuthModal } from '@/components/auth-modal';
import { CreatePostModal } from '@/components/create-post-modal';
import { LoadMoreButton } from '@/components/load-more-button';
import { InboxModal } from '@/components/inbox-modal';
import { UserSettingsPage } from '@/components/user-settings-page';
import { SubredditWiki } from '@/components/subreddit-wiki';
import { api, Post, PostSort, CommentSort } from '@/lib/api';

type View =
  | { kind: 'home' }
  | { kind: 'popular' }
  | { kind: 'subreddit'; name: string }
  | { kind: 'subreddit-wiki'; name: string }
  | { kind: 'multireddit'; name: string }
  | { kind: 'post'; id: string }
  | { kind: 'user'; name: string }
  | { kind: 'search'; q: string }
  | { kind: 'create-subreddit' };

function parseHash(): View {
  if (typeof window === 'undefined') return { kind: 'home' };
  const raw = window.location.hash.slice(1); // remove '#'
  if (!raw) return { kind: 'home' };
  // Strip any query string the user/caller appended after the hash route.
  const h = raw.split('?')[0].split('&')[0];
  if (!h) return { kind: 'home' };
  if (h.startsWith('/r/')) {
    const parts = h.slice(3).split('/');
    const name = parts[0];
    if (!name) return { kind: 'home' };
    if (parts[1] === 'wiki') return { kind: 'subreddit-wiki', name };
    return { kind: 'subreddit', name };
  }
  if (h.startsWith('/m/')) {
    const name = h.slice(3).split('/')[0];
    if (name) return { kind: 'multireddit', name };
    return { kind: 'home' };
  }
  if (h.startsWith('/post/')) {
    const id = h.slice(6);
    if (id) return { kind: 'post', id };
    return { kind: 'home' };
  }
  if (h.startsWith('/user/')) {
    const name = h.slice(6);
    if (name) return { kind: 'user', name };
    return { kind: 'home' };
  }
  if (h.startsWith('/search')) {
    // search query string is part of the hash, e.g. /search?q=foo
    const qs = raw.split('?')[1] || '';
    const params = new URLSearchParams(qs);
    const q = params.get('q');
    if (q) return { kind: 'search', q };
    return { kind: 'home' };
  }
  if (h === '/popular') return { kind: 'popular' };
  if (h === '/create-subreddit') return { kind: 'create-subreddit' };
  return { kind: 'home' };
}

function setHash(v: View) {
  let h = '';
  switch (v.kind) {
    case 'home': h = ''; break;
    case 'popular': h = '/popular'; break;
    case 'subreddit': h = `/r/${v.name}`; break;
    case 'subreddit-wiki': h = `/r/${v.name}/wiki`; break;
    case 'multireddit': h = `/m/${v.name}`; break;
    case 'post': h = `/post/${v.id}`; break;
    case 'user': h = `/user/${v.name}`; break;
    case 'search': h = `/search?q=${encodeURIComponent(v.q)}`; break;
    case 'create-subreddit': h = '/create-subreddit'; break;
  }
  if (window.location.hash.slice(1) !== h) {
    window.location.hash = h;
  }
}

export default function Home() {
  const [view, setView] = useState<View>({ kind: 'home' });
  const [sort, setSort] = useState<PostSort>('hot');
  const [timeFilter, setTimeFilter] = useState<'hour' | 'day' | 'week' | 'month' | 'year' | 'all'>('all');
  const [commentSort, setCommentSort] = useState<CommentSort>('best');
  const [authModal, setAuthModal] = useState<null | 'login' | 'register'>(null);
  const [createPostOpen, setCreatePostOpen] = useState(false);
  const [inboxOpen, setInboxOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [crosspostParent, setCrosspostParent] = useState<{ id: string; title: string } | null>(null);
  const [theme, setTheme] = useState<'light' | 'dark'>('dark');
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const qc = useQueryClient();

  // Init theme from localStorage
  useEffect(() => {
    const t = (localStorage.getItem('freeddit-theme') as 'light' | 'dark') || 'dark';
    setTheme(t);
    document.documentElement.classList.toggle('dark', t === 'dark');
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme((t) => {
      const next = t === 'dark' ? 'light' : 'dark';
      document.documentElement.classList.toggle('dark', next === 'dark');
      localStorage.setItem('freeddit-theme', next);
      return next;
    });
  }, []);

  // Sync with hash
  useEffect(() => {
    setView(parseHash());
    const onHashChange = () => {
      setView(parseHash());
      window.scrollTo(0, 0);
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  // Re-fetch comments when added
  useEffect(() => {
    const onCommentAdded = (e: Event) => {
      const detail = (e as CustomEvent).detail as { postId: string };
      qc.invalidateQueries({ queryKey: ['comments', detail.postId, commentSort] });
      qc.invalidateQueries({ queryKey: ['post', detail.postId] });
    };
    window.addEventListener('comment-added', onCommentAdded as EventListener);
    // Re-fetch post when edited
    const onPostEdited = (e: Event) => {
      const detail = (e as CustomEvent).detail as { postId: string };
      qc.invalidateQueries({ queryKey: ['post', detail.postId] });
      qc.invalidateQueries({ queryKey: ['posts'] });
    };
    window.addEventListener('post-edited', onPostEdited as EventListener);
    // Crosspost trigger: opens create-post modal with parent set
    const onCrosspost = (e: Event) => {
      const detail = (e as CustomEvent).detail as { postId: string; title: string };
      setCrosspostParent({ id: detail.postId, title: detail.title });
      setCreatePostOpen(true);
    };
    window.addEventListener('crosspost-trigger', onCrosspost as EventListener);
    return () => {
      window.removeEventListener('comment-added', onCommentAdded as EventListener);
      window.removeEventListener('post-edited', onPostEdited as EventListener);
      window.removeEventListener('crosspost-trigger', onCrosspost as EventListener);
    };
  }, [commentSort, qc]);

  const navigate = useCallback((v: View) => {
    setHash(v);
    setView(v);
    window.scrollTo(0, 0);
    setSidebarOpen(false);
  }, []);

  const openPost = useCallback((post: Post) => {
    navigate({ kind: 'post', id: post.id });
  }, [navigate]);

  const openSubreddit = useCallback((name: string) => {
    navigate({ kind: 'subreddit', name });
    setSort('hot');
  }, [navigate]);

  const openUser = useCallback((name: string) => {
    navigate({ kind: 'user', name });
  }, [navigate]);

  const openSearch = useCallback((q: string) => {
    navigate({ kind: 'search', q });
  }, [navigate]);

  // === Data fetching ===
  const postsQuery = useQuery({
    queryKey: ['posts', view, sort, timeFilter],
    queryFn: async () => {
      if (view.kind === 'home') return api.listHome(sort, 25, sort === 'top' ? timeFilter : undefined);
      if (view.kind === 'popular') return api.listHome(sort, 25, sort === 'top' ? timeFilter : undefined);
      if (view.kind === 'subreddit') return api.listSubredditPosts(view.name, sort, 25, sort === 'top' ? timeFilter : undefined);
      return null;
    },
    enabled: view.kind === 'home' || view.kind === 'popular' || view.kind === 'subreddit',
  });

  const postQuery = useQuery({
    queryKey: ['post', (view as { id?: string }).id],
    queryFn: () => api.getPost((view as { id: string }).id),
    enabled: view.kind === 'post',
  });

  const commentsQuery = useQuery({
    queryKey: ['comments', (view as { id?: string }).id, commentSort],
    queryFn: () => api.listComments((view as { id: string }).id, commentSort),
    enabled: view.kind === 'post',
  });

  const subredditQuery = useQuery({
    queryKey: ['subreddit', (view as { name?: string }).name],
    queryFn: () => api.getSubreddit((view as { name: string }).name),
    enabled: view.kind === 'subreddit',
  });

  const posts = postsQuery.data?.data.children.map((c) => c.data) || [];
  const post = view.kind === 'post' ? postQuery.data?.data : null;
  const comments = view.kind === 'post' ? (commentsQuery.data?.data.children.map((c) => c.data) || []) : [];
  const subreddit = view.kind === 'subreddit' ? subredditQuery.data?.data : null;

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Header
        onSearch={openSearch}
        onCreatePost={() => { setCrosspostParent(null); setCreatePostOpen(true); }}
        onOpenUser={openUser}
        onToggleSidebar={() => setSidebarOpen((v) => !v)}
        onOpenLogin={() => setAuthModal('login')}
        onOpenRegister={() => setAuthModal('register')}
        onOpenInbox={() => setInboxOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        theme={theme}
        onToggleTheme={toggleTheme}
      />

      <div className="flex max-w-7xl mx-auto">
        <Sidebar
          onOpenHome={() => navigate({ kind: 'home' })}
          onOpenPopular={() => navigate({ kind: 'popular' })}
          onOpenSubreddit={openSubreddit}
          onCreateSubreddit={() => navigate({ kind: 'create-subreddit' })}
        />

        <main className="flex-1 min-w-0 p-3 max-w-3xl mx-auto">
          {/* Sort tabs for feed views */}
          {(view.kind === 'home' || view.kind === 'popular' || view.kind === 'subreddit') && (
            <>
              {view.kind === 'subreddit' && subreddit && (
                <SubredditHeader
                  subreddit={subreddit}
                  onOpenCreatePost={() => setCreatePostOpen(true)}
                />
              )}
              <div className="sticky top-12 z-30 bg-background/95 backdrop-blur py-2 mb-2">
                <SortTabs
                  value={sort}
                  onChange={setSort}
                  showTimeFilter={true}
                  timeFilter={timeFilter}
                  onTimeFilterChange={setTimeFilter}
                />
              </div>
            </>
          )}

          {/* Create subreddit view */}
          {view.kind === 'create-subreddit' && <CreateSubredditForm onCreated={openSubreddit} />}

          {/* Feed */}
          {(view.kind === 'home' || view.kind === 'popular' || view.kind === 'subreddit') && (
            <div className="space-y-2">
              {postsQuery.isLoading ? (
                [1, 2, 3, 4, 5].map((i) => (
                  <div key={i} className="h-28 bg-accent/30 animate-pulse rounded-lg" />
                ))
              ) : posts.length === 0 ? (
                <div className="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
                  <p>No posts yet.</p>
                  {view.kind === 'subreddit' && (
                    <button
                      onClick={() => setCreatePostOpen(true)}
                      className="mt-3 px-4 py-2 rounded-full bg-orange-500 text-white text-sm font-semibold hover:bg-orange-600"
                    >
                      Be the first to post
                    </button>
                  )}
                </div>
              ) : (
                <>
                  {posts.map((p) => (
                    <PostCard
                      key={p.id}
                      post={p}
                      onOpenPost={openPost}
                      onOpenSubreddit={openSubreddit}
                      onOpenUser={openUser}
                    />
                  ))}
                  <LoadMoreButton
                    hasMore={posts.length >= 25}
                    onLoadMore={async () => {
                      // Simple "load more" — fetch next page using after=last_post_id
                      const lastPost = posts[posts.length - 1];
                      if (!lastPost) return;
                      try {
                        const next = view.kind === 'subreddit'
                          ? await api.listSubredditPosts(view.name, sort, 25, sort === 'top' ? timeFilter : undefined)
                          : await api.listHome(sort, 25, sort === 'top' ? timeFilter : undefined);
                        // Append: trigger refetch with bigger limit (simple approach)
                        qc.invalidateQueries({ queryKey: ['posts', view, sort, timeFilter] });
                      } catch (e) { /* ignore */ }
                    }}
                  />
                </>
              )}
            </div>
          )}

          {/* Post detail */}
          {view.kind === 'post' && (
            <>
              {postQuery.isLoading ? (
                <div className="h-64 bg-accent/30 animate-pulse rounded-lg" />
              ) : post ? (
                <PostDetail
                  post={post}
                  sort={commentSort}
                  onSortChange={setCommentSort}
                  comments={comments}
                  isLoadingComments={commentsQuery.isLoading}
                  onOpenSubreddit={openSubreddit}
                  onOpenUser={openUser}
                  onBack={() => navigate({ kind: 'home' })}
                />
              ) : (
                <div className="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
                  Post not found.
                </div>
              )}
            </>
          )}

          {/* User profile */}
          {view.kind === 'user' && (
            <UserPage
              username={view.name}
              onOpenPost={openPost}
              onOpenSubreddit={openSubreddit}
              onOpenUser={openUser}
            />
          )}

          {/* Search results */}
          {view.kind === 'search' && (
            <SearchPage
              query={view.q}
              onOpenPost={openPost}
              onOpenSubreddit={openSubreddit}
              onOpenUser={openUser}
            />
          )}

          {/* Subreddit wiki */}
          {view.kind === 'subreddit-wiki' && (
            <SubredditWiki subredditName={view.name} isMod={false /* TODO: check mod status */} />
          )}

          {/* Multireddit posts */}
          {view.kind === 'multireddit' && (
            <MultiredditView name={view.name} onOpenPost={openPost} onOpenSubreddit={openSubreddit} onOpenUser={openUser} />
          )}
        </main>
      </div>

      {/* Mobile sidebar drawer */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/60 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        >
          <div
            className="absolute left-0 top-0 bottom-0 w-72 bg-card border-r border-border overflow-y-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="h-12" />
            <Sidebar
              onOpenHome={() => navigate({ kind: 'home' })}
              onOpenPopular={() => navigate({ kind: 'popular' })}
              onOpenSubreddit={openSubreddit}
              onCreateSubreddit={() => navigate({ kind: 'create-subreddit' })}
              onClose={() => setSidebarOpen(false)}
            />
          </div>
        </div>
      )}

      {/* Modals */}
      {authModal && (
        <AuthModal
          mode={authModal}
          onClose={() => setAuthModal(null)}
          onSwitchMode={(m) => setAuthModal(m)}
        />
      )}
      {createPostOpen && (
        <CreatePostModal
          onClose={() => { setCreatePostOpen(false); setCrosspostParent(null); }}
          defaultSubreddit={view.kind === 'subreddit' ? view.name : undefined}
          crosspostParentId={crosspostParent?.id}
          crosspostParentTitle={crosspostParent?.title}
          onCreated={(path) => {
            const id = path.split('/').pop()!;
            navigate({ kind: 'post', id });
            setCrosspostParent(null);
          }}
        />
      )}
      {inboxOpen && (
        <InboxModal
          onClose={() => setInboxOpen(false)}
          onOpenPost={(id) => { setInboxOpen(false); navigate({ kind: 'post', id }); }}
          onOpenUser={(name) => { setInboxOpen(false); navigate({ kind: 'user', name }); }}
        />
      )}
      {settingsOpen && (
        <UserSettingsPage onClose={() => setSettingsOpen(false)} />
      )}
    </div>
  );
}

function CreateSubredditForm({ onCreated }: { onCreated: (name: string) => void }) {
  const { user } = useAuth();
  const [name, setName] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [description, setDescription] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.createSubreddit({ name, display_name: displayName || name, description });
      onCreated(name);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  if (!user) {
    return (
      <div className="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
        Please log in to create a community.
      </div>
    );
  }

  return (
    <div className="bg-card border border-border rounded-lg p-6">
      <h2 className="text-xl font-bold mb-4">Create a community</h2>
      {error && (
        <div className="bg-red-500/10 border border-red-500/30 text-red-600 dark:text-red-400 text-sm rounded-lg p-3 mb-4">
          {error}
        </div>
      )}
      <form onSubmit={submit} className="space-y-3">
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Name</label>
          <p className="text-xs text-muted-foreground mb-1">
            Community names cannot be changed. Choose wisely. 3-21 characters, letters/digits/_.
          </p>
          <div className="flex items-center bg-accent/30 border border-border rounded-lg">
            <span className="pl-3 text-sm text-muted-foreground">r/</span>
            <input
              type="text"
              required
              minLength={3}
              maxLength={21}
              value={name}
              onChange={(e) => setName(e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, ''))}
              className="flex-1 bg-transparent px-1 py-2 text-sm outline-none"
              placeholder="community_name"
            />
          </div>
        </div>
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Display Name</label>
          <input
            type="text"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500"
            placeholder="A friendly title for your community"
          />
        </div>
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Description</label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={3}
            className="w-full mt-1 bg-accent/30 border border-border rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 resize-y"
            placeholder="What's this community about?"
          />
        </div>
        <button
          type="submit"
          disabled={busy}
          className="bg-orange-500 hover:bg-orange-600 text-white font-semibold py-2 px-4 rounded-lg disabled:opacity-50"
        >
          {busy ? 'Creating…' : 'Create Community'}
        </button>
      </form>
    </div>
  );
}

function MultiredditView({ name, onOpenPost, onOpenSubreddit, onOpenUser }: { name: string; onOpenPost: (post: Post) => void; onOpenSubreddit: (n: string) => void; onOpenUser: (n: string) => void }) {
  const { data: multireddit } = useQuery({
    queryKey: ['multireddit', name],
    queryFn: () => api.getMultireddit(name),
  });
  const { data: postsData, isLoading } = useQuery({
    queryKey: ['multireddit-posts', name],
    queryFn: () => api.listMultiredditPosts(name),
  });

  if (!multireddit) return <div className="text-center p-8 text-muted-foreground">Loading multireddit…</div>;

  return (
    <div>
      <div className="bg-card border border-border rounded-lg p-4 mb-3">
        <h1 className="text-2xl font-bold">m/{name}</h1>
        <p className="text-xs text-muted-foreground mt-1">{multireddit.subreddit_ids.length} subreddits aggregated</p>
      </div>
      <div className="space-y-2">
        {isLoading ? (
          [1,2,3].map(i => <div key={i} className="h-28 bg-accent/30 animate-pulse rounded-lg" />)
        ) : (postsData?.data.children || []).length === 0 ? (
          <div className="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
            <p>This multireddit has no posts yet.</p>
            <p className="text-xs mt-1">Subscribe to some subreddits first, then add them to this multireddit.</p>
          </div>
        ) : (
          postsData?.data.children.map(({ data: p }) => (
            <PostCard
              key={p.id}
              post={p}
              onOpenPost={onOpenPost}
              onOpenSubreddit={onOpenSubreddit}
              onOpenUser={onOpenUser}
            />
          ))
        )}
      </div>
    </div>
  );
}
