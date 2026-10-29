-- Migration 003: Remaining Reddit features
-- Media uploads, multireddits, wiki, crossposts, 2FA, password reset, user settings

-- =========================================================
-- Media uploads (image / video / thumbnail)
-- =========================================================
CREATE TABLE IF NOT EXISTS media_uploads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    original_name TEXT,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    -- 'image' | 'video' | 'gif' | 'other'
    kind TEXT NOT NULL,
    -- local path under /home/z/my-project/uploads/
    storage_path TEXT NOT NULL,
    -- publicly served URL
    url TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    duration_sec REAL,
    sha256 TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_media_user ON media_uploads(user_id);

-- =========================================================
-- Multireddits / custom feeds (Reddit: r/user/m/feedname)
-- =========================================================
CREATE TABLE IF NOT EXISTS multireddits (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    visibility TEXT NOT NULL DEFAULT 'private',  -- 'private' | 'public'
    subreddit_ids TEXT NOT NULL DEFAULT '',  -- comma-separated subreddit IDs
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, name)
);
CREATE INDEX IF NOT EXISTS idx_multireddits_user ON multireddits(user_id);

-- =========================================================
-- Subreddit wiki (Reddit: r/sub/wiki)
-- =========================================================
CREATE TABLE IF NOT EXISTS subreddit_wiki_pages (
    id TEXT PRIMARY KEY,
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,  -- e.g. 'index', 'rules', 'faq'
    title TEXT NOT NULL,
    content TEXT NOT NULL,  -- markdown
    -- edit history (last 10 revisions, JSON array of {content, edited_by, edited_at})
    history TEXT NOT NULL DEFAULT '[]',
    last_editor_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(subreddit_id, slug)
);
CREATE INDEX IF NOT EXISTS idx_wiki_subreddit ON subreddit_wiki_pages(subreddit_id);

-- =========================================================
-- Crossposts (Reddit: posts that link to another post in another subreddit)
-- Added as a column on posts, but we also track the chain.
-- =========================================================
CREATE TABLE IF NOT EXISTS crosspost_links (
    id TEXT PRIMARY KEY,
    source_post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    target_post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE(source_post_id, target_post_id)
);
CREATE INDEX IF NOT EXISTS idx_crosspost_source ON crosspost_links(source_post_id);

-- =========================================================
-- User settings (Reddit: preferences, account settings)
-- =========================================================
CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- display prefs
    theme TEXT NOT NULL DEFAULT 'system',  -- 'light' | 'dark' | 'system'
    content_visibility TEXT NOT NULL DEFAULT 'safe',  -- 'safe' | 'nsfw'
    default_sort TEXT NOT NULL DEFAULT 'hot',
    default_comment_sort TEXT NOT NULL DEFAULT 'best',
    -- email prefs
    email_on_pm INTEGER NOT NULL DEFAULT 1,
    email_on_mention INTEGER NOT NULL DEFAULT 0,
    email_on_reply INTEGER NOT NULL DEFAULT 1,
    -- privacy
    show_online_status INTEGER NOT NULL DEFAULT 1,
    show_post_history INTEGER NOT NULL DEFAULT 1,
    show_comment_history INTEGER NOT NULL DEFAULT 1,
    -- 2FA
    totp_secret TEXT,
    totp_backup_codes TEXT,  -- JSON array of hashed codes
    totp_enabled INTEGER NOT NULL DEFAULT 0,
    -- email verification
    email_verified INTEGER NOT NULL DEFAULT 0,
    email_verification_token TEXT,
    email_verification_expires TEXT,
    -- password reset
    password_reset_token TEXT,
    password_reset_expires TEXT,
    -- updated_at
    updated_at TEXT NOT NULL
);

-- =========================================================
-- Subreddit posts: add crosspost_parent column
-- We need an ALTER since posts table already exists in migration 001.
-- SQLite supports ADD COLUMN.
-- =========================================================
ALTER TABLE posts ADD COLUMN crosspost_parent_id TEXT REFERENCES posts(id);
ALTER TABLE posts ADD COLUMN media_url TEXT;
ALTER TABLE posts ADD COLUMN media_kind TEXT;  -- 'image' | 'video' | 'gif'
