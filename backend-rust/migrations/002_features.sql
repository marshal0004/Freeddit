-- Migration 002: Add missing Reddit features (saves, hides, reports, rules, mod actions, sticky comments)

-- =========================================================
-- Saved posts (Reddit: Save post feature)
-- =========================================================
CREATE TABLE IF NOT EXISTS saved_posts (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, post_id)
);
CREATE INDEX IF NOT EXISTS idx_saved_user ON saved_posts(user_id);

-- =========================================================
-- Hidden posts (Reddit: Hide post feature)
-- =========================================================
CREATE TABLE IF NOT EXISTS hidden_posts (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, post_id)
);
CREATE INDEX IF NOT EXISTS idx_hidden_user ON hidden_posts(user_id);

-- =========================================================
-- Reports (Reddit: Report post/comment feature)
-- =========================================================
CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY,
    reporter_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    target_type TEXT NOT NULL,  -- 'post' | 'comment'
    subreddit_id TEXT REFERENCES subreddits(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    details TEXT,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending | reviewed | dismissed | actioned
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reports_target ON reports(target_id);
CREATE INDEX IF NOT EXISTS idx_reports_subreddit ON reports(subreddit_id);

-- =========================================================
-- Subreddit rules (Reddit: community rules sidebar)
-- =========================================================
CREATE TABLE IF NOT EXISTS subreddit_rules (
    id TEXT PRIMARY KEY,
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    short_name TEXT NOT NULL,
    description TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_rules_subreddit ON subreddit_rules(subreddit_id);

-- =========================================================
-- Subreddit moderators (Reddit: explicit mod role table)
-- =========================================================
CREATE TABLE IF NOT EXISTS subreddit_moderators (
    subreddit_id TEXT NOT NULL REFERENCES subreddits(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions TEXT NOT NULL DEFAULT 'all',  -- 'all' | 'config' | 'posts' | 'access' | 'mail'
    added_at TEXT NOT NULL,
    PRIMARY KEY (subreddit_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_mods_subreddit ON subreddit_moderators(subreddit_id);

-- =========================================================
-- Message threads (Reddit: PM/inbox feature)
-- =========================================================
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    last_message_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversation_participants (
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TEXT NOT NULL,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_messages_user ON messages(sender_id);

-- =========================================================
-- Notifications (Reddit: bell icon notifications)
-- =========================================================
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,  -- 'comment_reply' | 'post_reply' | 'mention' | 'upvote' | 'award' | 'mod_action'
    target_id TEXT,
    target_type TEXT,  -- 'post' | 'comment'
    actor_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    body TEXT,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notifs_user ON notifications(user_id, is_read);
