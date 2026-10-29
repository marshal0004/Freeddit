// Freeddit API client — talks to the Rust backend via the Caddy XTransformPort gateway.
// In production this would be replaced with a direct fetch to the Rust API host.

export const API_PORT = 3001;

const TOKEN_KEY = 'freeddit_token';
const USER_KEY = 'freeddit_user';

export function getToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string | null, user: AuthUser | null) {
  if (typeof window === 'undefined') return;
  if (token) {
    localStorage.setItem(TOKEN_KEY, token);
    if (user) localStorage.setItem(USER_KEY, JSON.stringify(user));
  } else {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
  }
}

export function getStoredUser(): AuthUser | null {
  if (typeof window === 'undefined') return null;
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try { return JSON.parse(raw) as AuthUser; } catch { return null; }
}

export interface AuthUser {
  id: string;
  username: string;
  email?: string | null;
  is_admin: boolean;
  post_karma: number;
  comment_karma: number;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  user: AuthUser;
}

export interface User {
  id: string;
  name: string;
  display_name: string;
  email?: string | null;
  avatar?: string | null;
  banner_color?: string | null;
  bio?: string | null;
  post_karma: number;
  comment_karma: number;
  total_karma: number;
  is_admin: boolean;
  created_at: string;
}

export interface Subreddit {
  id: string;
  display_name: string;
  display_name_prefixed: string;
  title: string;
  description?: string | null;
  public_description?: string | null;
  sidebar?: string | null;
  icon?: string | null;
  banner?: string | null;
  primary_color?: string | null;
  subscribers: number;
  active_user_count: number;
  sub_type: string;
  over18: boolean;
  created_at: string;
}

export interface Post {
  id: string;
  name: string;
  subreddit_id: string;
  subreddit: string;
  subreddit_name_prefixed: string;
  author: string;
  author_id: string;
  title: string;
  url?: string | null;
  body?: string | null;
  kind: string;
  flair_text?: string | null;
  flair_color?: string | null;
  thumbnail?: string | null;
  score: number;
  ups: number;
  downs: number;
  upvote_ratio: number;
  num_comments: number;
  views: number;
  is_sticky: boolean;
  is_locked: boolean;
  is_removed: boolean;
  is_spoiler: boolean;
  over18: boolean;
  is_self: boolean;
  created_at: string;
  edited_at?: string | null;
  likes: number | null; // -1 | 0 | 1 | null
}

export interface Comment {
  id: string;
  name: string;
  post_id: string;
  subreddit_id: string;
  subreddit: string;
  author: string;
  author_id: string;
  parent_id?: string | null;
  depth: number;
  body: string;
  score: number;
  ups: number;
  downs: number;
  upvote_ratio: number;
  num_replies: number;
  is_sticky: boolean;
  is_locked: boolean;
  is_op: boolean;
  created_at: string;
  edited_at?: string | null;
  likes: number | null;
  replies: Comment[];
}

export interface Listing<T> {
  kind: string;
  data: {
    after?: string | null;
    before?: string | null;
    children: Array<{ kind: string; data: T }>;
    dist: number;
  };
}

export type PostSort = 'hot' | 'best' | 'new' | 'top' | 'rising' | 'controversial';
export type CommentSort = 'best' | 'top' | 'new' | 'controversial' | 'old' | 'qa';

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string> || {}),
  };
  if (token) headers['Authorization'] = `Bearer ${token}`;

  // The Caddy gateway requires XTransformPort=PORT as a query param.
  // Append it correctly whether or not the path already has query params.
  const separator = path.includes('?') ? '&' : '?';
  const url = `/api${path}${separator}XTransformPort=${API_PORT}`;

  const res = await fetch(url, {
    ...options,
    headers,
  });

  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const body = await res.json();
      msg = body.message || body.error || msg;
    } catch {}
    throw new Error(msg);
  }
  return res.json() as Promise<T>;
}

export const api = {
  health: () => apiFetch<{ ok: string }>('/health'),

  register: (username: string, password: string, email?: string) =>
    apiFetch<AuthResponse>('/register', {
      method: 'POST',
      body: JSON.stringify({ username, password, email }),
    }),

  login: (username: string, password: string) =>
    apiFetch<AuthResponse>('/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),

  me: () => apiFetch<{ id: string; username: string; is_admin: boolean }>('/me'),

  // Subreddits
  listSubreddits: (limit = 25) =>
    apiFetch<Listing<Subreddit>>(`/subreddits?limit=${limit}`),
  popularSubreddits: (limit = 25) =>
    apiFetch<Listing<Subreddit>>(`/subreddits/popular?limit=${limit}`),
  getSubreddit: (name: string) =>
    apiFetch<{ kind: string; data: Subreddit }>(`/subreddits/${name}`),
  createSubreddit: (data: { name: string; display_name: string; description?: string; sidebar?: string }) =>
    apiFetch<{ kind: string; data: Subreddit }>('/subreddits', {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  subscribe: (name: string, action: 'sub' | 'unsub') =>
    apiFetch<{ subscribed: boolean; subreddit: string }>(`/subreddits/${name}/subscribe`, {
      method: 'POST',
      body: JSON.stringify({ action }),
    }),

  // Posts
  listHome: (sort: PostSort = 'hot', limit = 25, t?: string) =>
    apiFetch<Listing<Post>>(`/posts?sort=${sort}&limit=${limit}${t ? `&t=${t}` : ''}`),
  listSubredditPosts: (name: string, sort: PostSort = 'hot', limit = 25, t?: string) =>
    apiFetch<Listing<Post>>(`/subreddits/${name}/posts?sort=${sort}&limit=${limit}${t ? `&t=${t}` : ''}`),
  getPost: (id: string) =>
    apiFetch<{ kind: string; data: Post }>(`/posts/${id}`),

  // Comments
  listComments: (postId: string, sort: CommentSort = 'best') =>
    apiFetch<Listing<Comment>>(`/posts/${postId}/comments?sort=${sort}`),
  createComment: (postId: string, body: string, parentId?: string) =>
    apiFetch<{ kind: string; data: Comment }>(`/posts/${postId}/comments`, {
      method: 'POST',
      body: JSON.stringify({ post_id: postId, body, parent_id: parentId }),
    }),

  // Posts (create)
  createPost: (data: {
    subreddit: string;
    title: string;
    url?: string;
    body?: string;
    kind?: string;
    is_nsfw?: boolean;
    is_spoiler?: boolean;
    flair_text?: string;
    flair_color?: string;
    media_url?: string;
    media_kind?: string;
    crosspost_parent_id?: string;
  }) =>
    apiFetch<{ kind: string; data: Post }>('/posts', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  // Votes
  vote: (targetId: string, direction: 1 | 0 | -1) =>
    apiFetch<{ changed: boolean; score: number; likes: number }>(`/vote`, {
      method: 'POST',
      body: JSON.stringify({ target_id: targetId, direction }),
    }),

  // Users
  getUser: (name: string) =>
    apiFetch<{ kind: string; data: User }>(`/users/${name}`),
  getUserPosts: (name: string, sort = 'new', limit = 25) =>
    apiFetch<Listing<Post>>(`/users/${name}/posts?sort=${sort}&limit=${limit}`),
  getUserComments: (name: string, sort = 'new', limit = 25) =>
    apiFetch<Listing<Comment>>(`/users/${name}/comments?sort=${sort}&limit=${limit}`),

  // Search
  search: (q: string, type?: 'link' | 'sr' | 'user', limit = 25) =>
    apiFetch<Listing<Post | Subreddit | User>>(`/search?q=${encodeURIComponent(q)}&limit=${limit}${type ? `&type=${type}` : ''}`),

  // ===== Saves + hides =====
  savePost: (postId: string) =>
    apiFetch<{ saved: boolean; post_id: string }>(`/save/${postId}`, { method: 'POST' }),
  unsavePost: (postId: string) =>
    apiFetch<{ saved: boolean; post_id: string }>(`/save/${postId}`, { method: 'DELETE' }),
  hidePost: (postId: string) =>
    apiFetch<{ hidden: boolean; post_id: string }>(`/hide/${postId}`, { method: 'POST' }),
  unhidePost: (postId: string) =>
    apiFetch<{ hidden: boolean; post_id: string }>(`/hide/${postId}`, { method: 'DELETE' }),
  listSaved: () =>
    apiFetch<{ saved_post_ids: string[] }>(`/saved`),
  listHidden: () =>
    apiFetch<{ hidden_post_ids: string[] }>(`/hidden`),

  // ===== Reports =====
  createReport: (data: { target_id: string; target_type: 'post' | 'comment'; reason: string; details?: string; subreddit?: string }) =>
    apiFetch<{ report_id: string; status: string }>(`/reports`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  // ===== Edit / delete =====
  editPost: (id: string, data: {
    title?: string;
    body?: string;
    url?: string;
    is_nsfw?: boolean;
    is_spoiler?: boolean;
    flair_text?: string;
    flair_color?: string;
  }) =>
    apiFetch<{ kind: string; data: Post }>(`/posts/${id}/edit`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    }),
  deletePost: (id: string) =>
    apiFetch<{ deleted: boolean; post_id: string }>(`/posts/${id}/delete`, { method: 'DELETE' }),
  editComment: (id: string, body: string) =>
    apiFetch<{ kind: string; data: Comment }>(`/comments/${id}/edit`, {
      method: 'PATCH',
      body: JSON.stringify({ body }),
    }),
  deleteComment: (id: string) =>
    apiFetch<{ deleted: boolean; comment_id: string }>(`/comments/${id}/delete`, { method: 'DELETE' }),

  // ===== Mod tools =====
  modAction: (postId: string, subreddit: string, action: 'sticky' | 'unsticky' | 'lock' | 'unlock' | 'remove' | 'approve') =>
    apiFetch<{ post_id: string; action: string; applied: boolean }>(`/posts/${postId}/mod`, {
      method: 'POST',
      body: JSON.stringify({ subreddit, action }),
    }),
  stickyComment: (commentId: string, subreddit: string, action: 'sticky' | 'unsticky') =>
    apiFetch<{ comment_id: string; is_sticky: boolean }>(`/comments/${commentId}/sticky`, {
      method: 'POST',
      body: JSON.stringify({ subreddit, action }),
    }),
  // ===== Subreddit rules + moderators =====
  listRules: (subreddit: string) =>
    apiFetch<{ subreddit: string; rules: Array<{ id: string; short_name: string; description?: string | null; priority: number; created_at: string }> }>(`/subreddits/${subreddit}/rules`),
  createRule: (subreddit: string, data: { short_name: string; description?: string; priority?: number }) =>
    apiFetch<{ id: string; short_name: string; description?: string | null; priority: number; created_at: string }>(`/subreddits/${subreddit}/rules`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  deleteRule: (subreddit: string, ruleId: string) =>
    apiFetch<{ deleted: boolean; rule_id: string }>(`/subreddits/${subreddit}/rules/${ruleId}`, { method: 'DELETE' }),
  listModerators: (subreddit: string) =>
    apiFetch<{ subreddit: string; moderators: Array<{ username: string; permissions: string; added_at: string }> }>(`/subreddits/${subreddit}/moderators`),

  // ===== Media uploads (image/video) =====
  uploadMedia: async (file: File): Promise<{ id: string; url: string; kind: string; mime_type: string; size: number; sha256: string; filename: string; created_at: string }> => {
    const token = getToken();
    const formData = new FormData();
    formData.append('file', file);
    const sep = '?';
    const res = await fetch(`/api/media/upload${sep}XTransformPort=${API_PORT}`, {
      method: 'POST',
      headers: token ? { Authorization: `Bearer ${token}` } : {},
      body: formData,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ message: res.statusText }));
      throw new Error(err.message || `HTTP ${res.status}`);
    }
    return res.json();
  },
  listMyMedia: () =>
    apiFetch<{ media: Array<{ id: string; url: string; kind: string; mime_type: string; size: number; created_at: string }> }>(`/media/mine`),

  // ===== Multireddits =====
  listMultireddits: () =>
    apiFetch<{ multireddits: Array<{ id: string; name: string; description?: string | null; visibility: string; subreddit_ids: string[]; created_at: string; updated_at: string }> }>(`/multireddits`),
  createMultireddit: (data: { name: string; description?: string; visibility?: string; subreddits: string[] }) =>
    apiFetch<{ id: string; name: string; description?: string; visibility: string; subreddits: string[] }>(`/multireddits`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  getMultireddit: (name: string) =>
    apiFetch<{ id: string; name: string; visibility: string; subreddit_ids: string[]; created_at: string }>(`/multireddits/${name}`),
  deleteMultireddit: (name: string) =>
    apiFetch<{ deleted: boolean; name: string }>(`/multireddits/${name}`, { method: 'DELETE' }),
  listMultiredditPosts: (name: string) =>
    apiFetch<Listing<Post>>(`/multireddits/${name}/posts`),

  // ===== Notifications =====
  listNotifications: (unreadOnly = false) =>
    apiFetch<{ notifications: Notification[]; unread_count: number }>(`/notifications${unreadOnly ? '?unread_only=true' : ''}`),
  markNotificationRead: (notifId: string) =>
    apiFetch<{ marked_read: boolean }>(`/notifications/${notifId}/read`, { method: 'POST' }),
  markAllNotificationsRead: () =>
    apiFetch<{ marked_all_read: boolean }>(`/notifications/read-all`, { method: 'POST' }),

  // ===== Messages (DMs) =====
  startConversation: (data: { to: string; subject?: string; body: string }) =>
    apiFetch<{ conversation_id: string; message_id: string }>(`/messages`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  listConversations: () =>
    apiFetch<{ conversations: Array<{ id: string; other_participants: string[]; last_message_at: string; unread_count: number; last_message_preview: string }> }>(`/messages/conversations`),
  getConversation: (convId: string) =>
    apiFetch<{ conversation_id: string; messages: Array<{ id: string; sender_id: string; sender_name: string; body: string; is_read: boolean; created_at: string; is_mine: boolean }> }>(`/messages/conversations/${convId}`),
  sendMessage: (convId: string, body: string) =>
    apiFetch<{ message_id: string; conversation_id: string; body: string; sent_at: string }>(`/messages/conversations/${convId}/send`, {
      method: 'POST',
      body: JSON.stringify({ body }),
    }),
  deleteConversation: (convId: string) =>
    apiFetch<{ deleted: boolean; conversation_id: string }>(`/messages/conversations/${convId}`, { method: 'DELETE' }),

  // ===== Subreddit wiki =====
  listWikiPages: (subreddit: string) =>
    apiFetch<{ subreddit: string; pages: Array<{ slug: string; title: string; updated_at: string }> }>(`/subreddits/${subreddit}/wiki`),
  getWikiPage: (subreddit: string, slug: string) =>
    apiFetch<{ id: string; subreddit: string; slug: string; title: string; content: string; history: string; updated_at: string }>(`/subreddits/${subreddit}/wiki/${slug}`),
  upsertWikiPage: (subreddit: string, data: { slug: string; title: string; content: string }) =>
    apiFetch<{ id: string; subreddit: string; slug: string; title: string; content: string; updated_at: string }>(`/subreddits/${subreddit}/wiki/${data.slug}`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),
  deleteWikiPage: (subreddit: string, slug: string) =>
    apiFetch<{ deleted: boolean }>(`/subreddits/${subreddit}/wiki/${slug}`, { method: 'DELETE' }),

  // ===== Account / security =====
  getSettings: () =>
    apiFetch<{ theme: string; content_visibility: string; default_sort: string; default_comment_sort: string; email_on_pm: boolean; email_on_mention: boolean; email_on_reply: boolean; show_online_status: boolean; show_post_history: boolean; show_comment_history: boolean; two_factor_enabled: boolean; email_verified: boolean }>(`/account/settings`),
  updateSettings: (data: {
    theme?: string; content_visibility?: string; default_sort?: string; default_comment_sort?: string;
    email_on_pm?: boolean; email_on_mention?: boolean; email_on_reply?: boolean;
    show_online_status?: boolean; show_post_history?: boolean; show_comment_history?: boolean;
  }) =>
    apiFetch<{ updated: boolean; updated_at: string }>(`/account/settings`, { method: 'PATCH', body: JSON.stringify(data) }),
  updateProfile: (data: { avatar?: string; banner_color?: string; bio?: string; email?: string }) =>
    apiFetch<{ updated: boolean; updated_at: string }>(`/account/profile`, { method: 'PATCH', body: JSON.stringify(data) }),
  deleteAccount: (password: string) =>
    apiFetch<{ deleted: boolean; message: string }>(`/account/delete`, { method: 'POST', body: JSON.stringify({ password }) }),
  enable2FA: (code?: string) =>
    apiFetch<{ enabled: boolean; secret?: string; qr_url?: string; manual_entry?: string; backup_codes?: string[]; warning?: string }>(`/account/2fa/enable`, { method: 'POST', body: JSON.stringify(code ? { code } : {}) }),
  disable2FA: (code: string) =>
    apiFetch<{ disabled: boolean }>(`/account/2fa/disable`, { method: 'POST', body: JSON.stringify({ code }) }),
  get2FAStatus: () =>
    apiFetch<{ two_factor_enabled: boolean }>(`/account/2fa/status`),
  sendEmailVerification: (email?: string) =>
    apiFetch<{ sent: boolean; email: string; expires_at: string; dev_note: string }>(`/account/send-verification`, { method: 'POST', body: JSON.stringify({ email }) }),
  verifyEmail: (token: string) =>
    apiFetch<{ verified: boolean; user_id: string; message: string }>(`/verify-email?token=${encodeURIComponent(token)}`),
  requestPasswordReset: (usernameOrEmail: string) =>
    apiFetch<{ sent: boolean; note: string; token?: string; dev_note?: string }>(`/account/request-password-reset`, { method: 'POST', body: JSON.stringify({ username_or_email: usernameOrEmail }) }),
  confirmPasswordReset: (token: string, newPassword: string) =>
    apiFetch<{ reset: boolean; message: string }>(`/account/confirm-password-reset`, { method: 'POST', body: JSON.stringify({ token, new_password: newPassword }) }),
};

export interface Notification {
  id: string;
  kind: string;
  target_id: string | null;
  target_type: string | null;
  actor_id: string | null;
  actor_username: string | null;
  body: string | null;
  is_read: boolean;
  created_at: string;
}
