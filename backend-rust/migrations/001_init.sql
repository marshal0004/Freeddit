-- Freeddit database schema (SQLite for dev; PostgreSQL + Cassandra in production)
-- Faithful re-implementation of Reddit's data model:
--   - Users (Account service in Reddit)
--   - Subreddits (Subreddit service)
--   - Posts/Links (Link service — t3 in Reddit's old "thing" model)
--   - Comments (Comment service — t1)
--   - Votes (Vote service — Cassandra in Reddit prod)
--   - Subreddit subscriptions
--   - Awards (Awards service)

-- =========================================================
-- Users (Reddit: Account service / Postgres)
-- =========================================================
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,                       -- base36 fullname "t2_xxxx"
    username TEXT NOT NULL UNIQUE,
    email TEXT,
    password_hash TEXT NOT NULL,
    avatar TEXT,                                -- URL or color seed
    banner_color TEXT,
    bio TEXT,
    post_karma INTEGER NOT NULL DEFAULT 1,
    comment_karma INTEGER NOT NULL DEFAULT 1,
    is_admin INTEGER NOT NULL DEFAULT 0,
    is_moderator INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,                   -- ISO 8601 UTC
    updated_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- =========================================================
-- Subreddits (Reddit: Subreddit service / Postgres)
-- =========================================================
CREATE TABLE IF NOT EXISTS subreddits (
    id TEXT PRIMARY KEY,                        -- base36 fullname "t5_xxxx"
    name TEXT NOT NULL UNIQUE,                  -- display name (r/name)
    display_name TEXT NOT NULL,                 -- human title
    description TEXT,
    sidebar TEXT,                               -- markdown sidebar
    public_description TEXT,
    icon TEXT,
    banner TEXT,
    primary_color TEXT,
    subscribers INTEGER NOT NULL DEFAULT 0,
    active_users INTEGER NOT NULL DEFAULT 0,
    type TEXT NOT NULL DEFAULT 'public',         -- public/restricted/private
    over18 INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    creator_id TEXT NOT NULL REFERENCES users(id)
);
CREATE INDEX IF NOT EXISTS idx_subreddits_name ON subreddits(name);

-- =========================================================
-- Subreddit subscriptions (Reddit: Subscription service / Postgres)
-- =========================================================
CREATE TABLE IF NOT EXISTS subscriptions (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'subscriber',     -- subscriber/moderator/contributor
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, subreddit_id)
);

-- =========================================================
-- Posts (Reddit: Link service / Postgres — t3)
-- =========================================================
CREATE TABLE IF NOT EXISTS posts (
    id TEXT PRIMARY KEY,                        -- base36 fullname "t3_xxxx"
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    url TEXT,                                    -- empty for self posts
    body TEXT,                                   -- markdown body for self posts
    kind TEXT NOT NULL DEFAULT 'link',            -- link/self/image/video/gallery
    flair_text TEXT,
    flair_color TEXT,
    thumbnail TEXT,
    -- Reddit's vote fields (Hot algorithm uses these)
    score INTEGER NOT NULL DEFAULT 1,
    ups INTEGER NOT NULL DEFAULT 1,
    downs INTEGER NOT NULL DEFAULT 0,
    upvote_ratio REAL NOT NULL DEFAULT 1.0,
    -- ranking scores (precomputed on insert/update)
    hot_score REAL NOT NULL DEFAULT 0,
    best_score REAL NOT NULL DEFAULT 0,
    controversial_score REAL NOT NULL DEFAULT 0,
    -- counts
    num_comments INTEGER NOT NULL DEFAULT 0,
    views INTEGER NOT NULL DEFAULT 0,
    -- flags
    is_sticky INTEGER NOT NULL DEFAULT 0,
    is_locked INTEGER NOT NULL DEFAULT 0,
    is_removed INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_spoiler INTEGER NOT NULL DEFAULT 0,
    is_nsfw INTEGER NOT NULL DEFAULT 0,
    -- timestamps
    created_at TEXT NOT NULL,                    -- ISO 8601 UTC
    updated_at TEXT NOT NULL,
    -- for pagination
    rank_epoch REAL NOT NULL DEFAULT 0           -- seconds since epoch (for cursor pagination)
);
CREATE INDEX IF NOT EXISTS idx_posts_subreddit ON posts(subreddit_id);
CREATE INDEX IF NOT EXISTS idx_posts_author ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_hot ON posts(subreddit_id, hot_score DESC);
CREATE INDEX IF NOT EXISTS idx_posts_created ON posts(created_at DESC);

-- =========================================================
-- Comments (Reddit: Comment service / Postgres — t1)
-- Nested via adjacency list + materialized path for fast tree reads
-- =========================================================
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,                         -- base36 fullname "t1_xxxx"
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id TEXT,                               -- NULL = top-level comment
    path TEXT NOT NULL,                           -- materialized path: /root_id/parent_id/ (slash-separated)
    depth INTEGER NOT NULL DEFAULT 0,
    body TEXT NOT NULL,                           -- markdown
    -- vote fields (same shape as posts)
    score INTEGER NOT NULL DEFAULT 1,
    ups INTEGER NOT NULL DEFAULT 1,
    downs INTEGER NOT NULL DEFAULT 0,
    upvote_ratio REAL NOT NULL DEFAULT 1.0,
    hot_score REAL NOT NULL DEFAULT 0,
    best_score REAL NOT NULL DEFAULT 0,
    controversial_score REAL NOT NULL DEFAULT 0,
    num_replies INTEGER NOT NULL DEFAULT 0,
    is_sticky INTEGER NOT NULL DEFAULT 0,
    is_locked INTEGER NOT NULL DEFAULT 0,
    is_removed INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_op INTEGER NOT NULL DEFAULT 0,             -- 1 if author == post author
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_comments_post ON comments(post_id);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_id);
CREATE INDEX IF NOT EXISTS idx_comments_path ON comments(path);

-- =========================================================
-- Votes (Reddit: Vote service / Cassandra)
-- One row per (user, target) — same shape Reddit uses for vote storage
-- =========================================================
CREATE TABLE IF NOT EXISTS votes (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,                     -- post or comment id
    target_type TEXT NOT NULL,                    -- 'post' | 'comment'
    direction INTEGER NOT NULL,                   -- +1, 0, -1 (0 = cleared)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, target_id)
);
CREATE INDEX IF NOT EXISTS idx_votes_target ON votes(target_id, direction);

-- =========================================================
-- Awards (Reddit: Awards service)
-- =========================================================
CREATE TABLE IF NOT EXISTS awards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    color TEXT NOT NULL,
    coin_cost INTEGER NOT NULL,
    description TEXT
);

CREATE TABLE IF NOT EXISTS awardings (
    id TEXT PRIMARY KEY,
    target_id TEXT NOT NULL,
    target_type TEXT NOT NULL,                   -- 'post' | 'comment'
    award_id TEXT NOT NULL REFERENCES awards(id),
    giver_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    message TEXT
);
CREATE INDEX IF NOT EXISTS idx_awardings_target ON awardings(target_id);

-- =========================================================
-- Sessions (Reddit: session service)
-- =========================================================
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token TEXT NOT NULL UNIQUE,
    user_agent TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
