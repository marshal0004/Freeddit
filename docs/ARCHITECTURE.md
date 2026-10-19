# Freeddit — System Design

> An open-source Reddit clone with a Rust backend, Next.js + Electron frontend, and a faithful reimplementation of Reddit's ranking algorithms.

## Architecture (matches Reddit's published system design)

```
                       ┌───────────────────────┐
                       │   Electron shell      │
                       │  (desktop wrapper)    │
                       └──────────┬────────────┘
                                  │
                ┌─────────────────▼──────────────────┐
                │  Next.js 16 + React + Tailwind     │  ← Frontend (matches Reddit's React SPA)
                │  shadcn/ui · TanStack Query · JWT  │
                └─────────────────┬──────────────────┘
                                  │  REST + JSON
                                  ▼
                ┌──────────────────────────────────────┐
                │  Rust backend (axum 0.8)             │  ← Like Reddit's Rust services (since 2017)
                │  Services:                           │
                │   • AccountService  /api/register    │
                │   • SubredditService /api/subreddits │
                │   • LinkService     /api/posts       │
                │   • CommentService  /api/comments    │
                │   • VoteService     /api/vote        │
                │   • SearchService   /api/search      │
                │   • RankingEngine   (hot/best/etc)   │
                └────────┬────────────┬─────────────────┘
                         │            │
                         ▼            ▼
        ┌────────────────────┐   ┌──────────────────────┐
        │  PostgreSQL        │   │  Cassandra            │  ← Reddit's prod stack
        │  (users, subreddits,│   │  (votes, time-series, │     (dev: SQLite)
        │   posts, comments) │   │   counters)          │
        └────────────────────┘   └──────────────────────┘
                ▲                          ▲
                │                          │
        ┌───────┴──────────┐       ┌───────┴──────────┐
        │  Redis (cache)    │       │  Meilisearch      │  ← Reddit uses Redis + Lucene
        │  (dev: moka)       │       │  (dev: SQLite FTS)│
        └────────────────────┘       └──────────────────┘
```

## Reddit's Ranking Algorithms — faithfully reimplemented

All 6 ranking algorithms are implemented in `backend-rust/src/ranking.rs` exactly as Reddit published them.

### Hot (front page ranking)

```
hot(ups, downs, created) = sign * log10(max(|score|, 1)) + seconds_since_reddit_epoch / 45000
```

Where `seconds_since_reddit_epoch = created_at - 1134028003` (Reddit's first submission, 2005-12-08 7:46:43 AM UTC).

The `/45000` divisor is Reddit's choice: each vote ≈ 12.5 hours of newness, so the front page naturally rotates.

### Best (comment ranking — Wilson score)

```
best(ups, downs) = wilson_lower_bound(ups, ups+downs, z=1.96)
```

The Wilson score confidence interval lower bound for a Bernoulli parameter. Source: Evan Miller, "How Not To Sort By Average Rating" (2009). Reddit adopted this for comments in 2010. A 100-up/5-down comment ranks above 1-up/0-down.

### Controversial

```
controversial(ups, downs) = (ups + downs) / max(|ups - downs|, 1) * log10(ups + downs)
```

High when votes are many AND roughly split.

### Top

Pure `ups - downs` descending, with optional time-window filter (hour/day/week/month/year/all).

### New

Strictly `created_at DESC`. No math.

### Rising

Reddit's "vote velocity" approximation: `hot_score * (1 - age_in_hours/24) * (1 + (ups-downs)/10)`. Decays to zero after 24h.

## API shape — matches Reddit's public API

Every response uses Reddit's "Thing" + "Listing" wrapper:

```json
{
  "kind": "Listing",
  "data": {
    "after": null,
    "before": null,
    "dist": 25,
    "children": [
      { "kind": "t3", "data": { "id": "t3_abc", "name": "t3_abc", "title": "...", ... } }
    ]
  }
}
```

Thing kinds:
- `t1` — comment
- `t2` — account (user)
- `t3` — link (post)
- `t5` — subreddit

## Data model — matches Reddit's schema

- **users** (`t2`) — id, username, password_hash (Argon2id), karma (post + comment)
- **subreddits** (`t5`) — name, display_name, subscribers, type, primary_color
- **posts** (`t3`) — title, url OR body (self), kind, flair, score/ups/downs, ranking scores (hot/best/controversial), num_comments
- **comments** (`t1`) — body, parent_id (adjacency list), path (materialized path for fast tree reads), depth, vote fields
- **votes** — (user_id, target_id) → direction (+1/0/-1), target_type (post/comment)
- **subscriptions** — (user_id, subreddit_id) → role (subscriber/moderator/contributor)
- **sessions** — refresh tokens for JWT auth

## Auth — JWT + Argon2id (Reddit uses OAuth2 + bcrypt)

- Access tokens: 15-min JWT (Reddit: 1h)
- Refresh tokens: 30-day, stored in DB (Reddit: 1 year)
- Password hashing: Argon2id (stronger than Reddit's bcrypt)

## Production scale considerations

For Reddit-scale traffic (50M daily active users, ~1M requests/sec):

1. **PostgreSQL sharding** — Reddit shards by subreddit_id for posts/comments
2. **Cassandra for votes** — write-optimized for the vote service (millions of votes/day)
3. **Memcached/Redis layer** — cache hot posts, user karma, subreddit subscriber counts
4. **Elasticsearch/Meilisearch** — full-text search index (Reddit uses Lucene/Solr)
5. **CDN** — static assets, thumbnails, media (Reddit uses Fastly)
6. **Gossip protocol** — for cache invalidation across nodes (Reddit's "quorum" reads)
7. **Async queue** — RabbitMQ/Kafka for vote rollups, karma updates, notifications

## File structure

```
/home/z/my-project/
├── backend-rust/                  ← Rust backend
│   ├── Cargo.toml                 ← dependencies (axum, sqlx, tokio, argon2, jsonwebtoken)
│   ├── migrations/
│   │   └── 001_init.sql           ← full schema (users, subreddits, posts, comments, votes, sessions)
│   └── src/
│       ├── main.rs                ← axum server + AppState
│       ├── db.rs                  ← SQLite pool + migrations + seed trigger
│       ├── models.rs              ← Post, Comment, Subreddit, User, Thing, Listing
│       ├── auth.rs                ← JWT, Argon2, AuthUser / MaybeAuthUser extractors
│       ├── ranking.rs             ← Hot/Best/Top/New/Rising/Controversial algorithms (with tests)
│       ├── errors.rs              ← ApiError enum + IntoResponse
│       └── handlers/
│           ├── mod.rs             ← router
│           ├── auth.rs            ← register/login/me
│           ├── users.rs            ← user profile, posts, comments
│           ├── subreddits.rs       ← list, get, create, subscribe
│           ├── posts.rs            ← home feed, subreddit feed, get, create
│           ├── comments.rs         ← list (nested tree), get, create
│           ├── votes.rs            ← cast_vote (updates score + ranking)
│           ├── search.rs           ← search posts/subreddits/users
│           └── seed.rs             ← seed mock content (25 users, 10 subs, 32 posts, 471 comments)
│
├── src/                            ← Next.js frontend
│   ├── app/
│   │   ├── layout.tsx              ← providers (Auth + QueryClient), dark mode default
│   │   ├── page.tsx               ← SPA shell with hash router (home, subreddit, post, user, search)
│   │   └── globals.css
│   ├── lib/
│   │   ├── api.ts                 ← typed API client + XTransformPort gateway handling
│   │   ├── auth-context.tsx       ← React context for auth state
│   │   ├── format.ts              ← timeAgo, formatCount (k/M), colorFromString, initials
│   │   ├── db.ts                  ← Prisma client (unused; backend handles DB)
│   │   └── utils.ts               ← shadcn cn() helper
│   └── components/
│       ├── providers.tsx          ← QueryClientProvider + AuthProvider
│       ├── header.tsx             ← top nav: logo, search, login/signup, theme toggle, user menu
│       ├── sidebar.tsx            ← left rail: Home, Popular, Create, Popular Communities list
│       ├── sort-tabs.tsx          ← Hot/Best/New/Top/Rising/Controversial + time filter
│       ├── post-card.tsx          ← single post in feed
│       ├── post-detail.tsx        ← full post view with comment box
│       ├── comment-tree.tsx       ← nested comments with reply UI
│       ├── vote-button.tsx        ← upvote/downvote with optimistic updates
│       ├── subreddit-header.tsx   ← subreddit banner + Join button
│       ├── subreddit-avatar.tsx   ← colored circle with initials
│       ├── user-page.tsx          ← user profile with tabs (overview/posts/comments)
│       ├── search-page.tsx        ← search results (posts/communities/people)
│       ├── auth-modal.tsx         ← login/register modal
│       └── create-post-modal.tsx  ← create post form (post/link tabs)
│
├── scripts/
│   ├── backend-keepalive.sh       ← restart Rust backend if it crashes
│   ├── dev-keepalive.sh           ← restart Next.js dev if it crashes
│   └── freeddit-test.sh           ← self-contained smoke test
│
├── docs/
│   └── ARCHITECTURE.md            ← this file
│
├── freeddit.db                    ← SQLite database (dev; prod = Postgres + Cassandra)
└── Caddyfile                      ← Caddy gateway with XTransformPort routing
```

## Tech stack summary

| Layer | Dev | Production |
|---|---|---|
| Frontend framework | Next.js 16 + React 19 | Same |
| Styling | Tailwind CSS 4 + shadcn/ui | Same |
| State | TanStack Query + React context | Same + Zustand for client state |
| Desktop shell | Electron | Electron + auto-updates |
| Backend language | Rust 1.99 | Same |
| Web framework | axum 0.8 | Same |
| Async runtime | tokio | Same |
| Auth | JWT (jsonwebtoken 9.3) + Argon2id (argon2 0.5) | Same + OAuth2 |
| Primary DB | SQLite (sqlx 0.8) | PostgreSQL (sqlx) |
| Vote store | SQLite (same) | Cassandra / ScyllaDB |
| Cache | moka in-process | Redis / Memcached |
| Search | SQLite LIKE | Meilisearch / Elasticsearch |
| Reverse proxy | Caddy | Caddy / nginx |
| Process supervisor | bash keepalive scripts | systemd / Kubernetes |

## What works in v1

✅ User registration + login (JWT, Argon2id)
✅ Subreddit creation, subscription, listing
✅ Post creation (text/link), listing by sort (hot/best/new/top/rising/controversial)
✅ Time-windowed Top sort (hour/day/week/month/year/all)
✅ Comment creation (nested, materialized path for tree reads)
✅ Comment sort (best/top/new/controversial/old/qa)
✅ Voting (upvote/downvote/clear, atomic score + ranking recomputation)
✅ User profiles with karma, post history, comment history
✅ Search across posts, subreddits, and users
✅ Dark/light theme toggle
✅ Responsive layout (mobile sidebar drawer, desktop two-column)
✅ Hash-based routing for shareable URLs (#/r/name, #/post/id, #/user/name, #/search?q=)
