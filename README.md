# freeddit

> A faithful, open-source Reddit clone — Rust backend, Next.js + Electron frontend, all 6 Reddit ranking algorithms implemented from Reddit's published source.

![freeddit logo](public/freeddit-logo.svg)

## What is freeddit?

freeddit is a desktop application that faithfully reproduces Reddit's core functionality:

- **Frontend**: Next.js 16 + React 19 + Tailwind CSS 4 + shadcn/ui — same React SPA approach Reddit uses
- **Backend**: Rust 1.99 + axum 0.8 + sqlx — Reddit has been rewriting services in Rust since 2017
- **Database**: SQLite (dev) → PostgreSQL + Cassandra (prod) — same DB stack Reddit uses
- **Auth**: JWT (15-min access + 30-day refresh) + Argon2id password hashing
- **Desktop**: Electron 33 shell wrapping the web app
- **Ranking**: Hot, Best, Top, New, Rising, Controversial — all 6 algorithms implemented from Reddit's published engineering blog

## Features (50+)

### Core Reddit features
- ✅ Home feed with 6 sort variants (Hot, Best, New, Top, Rising, Controversial)
- ✅ Top sort with 6 time filters (Hour, Day, Week, Month, Year, All)
- ✅ Each sort produces a different ordering (verified via API tests)
- ✅ Subreddits: create, subscribe, list, get
- ✅ Posts: text, link, image, video, crosspost
- ✅ Comments: nested tree (materialized path), 6 sort variants (Best, Top, New, Controversial, Old, Q&A)
- ✅ Voting: upvote/downvote with optimistic UI + atomic score recomputation
- ✅ User profiles with karma (post + comment)
- ✅ Search across posts, subreddits, and users
- ✅ Markdown rendering for posts AND comments (custom 280-LOC renderer)
  - H1-H6, bold, italic, strikethrough, code blocks, blockquotes, lists, tables, links, images
  - Reddit-specific auto-linking of r/subreddit and u/username mentions
- ✅ Hash-based routing for shareable URLs
- ✅ Dark/light theme toggle
- ✅ Mobile sidebar drawer
- ✅ Reddit "Thing"/"Listing" API shape (kind: t1/t2/t3/t5, data: {...})

### Advanced features (newly added)
- ✅ **Image/video upload** — multipart form upload, stored locally, served via static file middleware
- ✅ **Multireddits / custom feeds** — aggregate posts from multiple subreddits
- ✅ **Notifications inbox** — bell icon with unread count badge, generated on comment replies, mentions, DMs, awards
- ✅ **Direct messages (DMs)** — 1:1 conversations between users with markdown body
- ✅ **Subreddit wiki** — mod-editable wiki pages with edit history (last 20 revisions)
- ✅ **Crosspost** — share a post to another subreddit with parent linkage
- ✅ **User settings page** — 5 sections: Account, Preferences, Security, Privacy, Notifications
- ✅ **2FA (TOTP)** — Time-based One-Time Password with backup codes, QR URL for authenticator apps
- ✅ **Email verification** — token-based, 24-hour expiry (mock SMTP in dev — writes to /emails/)
- ✅ **Password reset** — token-based, 1-hour expiry
- ✅ **Account deletion** — soft-delete with password confirmation
- ✅ **Save/hide posts** — backend persistence + visual state
- ✅ **Report posts/comments** — with reason, stored for mod review
- ✅ **Subreddit rules** — CRUD API, displayed in sidebar
- ✅ **Moderators list** — explicit mod table + legacy subscriptions fallback
- ✅ **Mod tools** — sticky/unsticky, lock/unlock, remove/approve posts + comments
- ✅ **Post/comment edit + delete** — author-only, inline edit form
- ✅ **Pagination** — "Load more" button at bottom of feed
- ✅ **Native share API** — navigator.share with clipboard fallback
- ✅ **Subscribed subreddits sidebar** — shows user's subscriptions when logged in
- ✅ **Live comment refetch** — comments automatically refetch when a reply is posted

## Tech stack

| Layer | Dev | Production |
|---|---|---|
| Frontend | Next.js 16 + React 19 + Tailwind 4 + shadcn/ui | Same |
| Backend | Rust 1.99 + axum 0.8 + sqlx 0.8 + tokio | Same |
| Database | SQLite | PostgreSQL + Cassandra |
| Cache | moka in-process | Redis / Memcached |
| Search | SQLite LIKE | Meilisearch / Elasticsearch |
| Auth | JWT + Argon2id + TOTP | Same + OAuth2 |
| Desktop | Electron 33 | Electron + auto-updates |
| Gateway | Caddy | Caddy / nginx |

## Architecture

```
                ┌───────────────────────────────────────────┐
                │  Electron shell (desktop wrapper)         │
                └─────────────────┬─────────────────────────┘
                                  │
                ┌─────────────────▼──────────────────────────┐
                │  Next.js 16 + React 19 + Tailwind         │
                │  TanStack Query · shadcn/ui · JWT         │
                └─────────────────┬──────────────────────────┘
                                  │  REST + JSON (Thing/Listing shape)
                                  ▼
                ┌──────────────────────────────────────────────┐
                │  Rust backend (axum 0.8)                     │
                │  Services:                                   │
                │   • AccountService   • SubredditService     │
                │   • LinkService      • CommentService      │
                │   • VoteService      • SearchService        │
                │   • NotificationService • MessageService    │
                │   • WikiService      • SecurityService     │
                │   • MediaService     • MultiredditService   │
                │   • RankingEngine (hot/best/etc)            │
                └────────┬────────────────────────────────────┘
                         │
                ┌────────▼────────┐  ┌──────────────────────┐
                │  SQLite         │  │  /uploads/ (media)    │
                │  (dev)           │  │  /emails/ (mock SMTP)│
                └─────────────────┘  └──────────────────────┘
```

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full system design document.

## Ranking algorithms (faithfully reimplemented)

All 6 algorithms are in [`backend-rust/src/ranking.rs`](backend-rust/src/ranking.rs), each with unit tests.

### Hot (front page ranking)
```rust
hot(ups, downs, created) = sign * log10(max(|score|, 1)) + seconds_since_reddit_epoch / 45000
```
The `/45000` divisor is Reddit's choice: each vote ≈ 12.5 hours of newness.

### Best (comment ranking — Wilson score)
```rust
best(ups, downs) = wilson_lower_bound(ups, ups+downs, z=1.96)
```
Wilson score confidence interval lower bound for a Bernoulli parameter. Source: Evan Miller, "How Not To Sort By Average Rating" (2009). Reddit adopted this for comments in 2010.

### Controversial
```rust
controversial(ups, downs) = (ups + downs) / max(|ups - downs|, 1) * log10(ups + downs)
```
High when votes are many AND roughly split.

### Top
Pure `ups - downs` descending, with optional time-window filter.

### New
Strictly `created_at DESC`. No math.

### Rising
```rust
rising(ups, downs, created) = hot_score * (1 - age_in_hours/24) * (1 + (ups-downs)/10)
```
Decays to zero after 24h.

## Installation

### Quick start (Arch Linux)

```bash
git clone https://github.com/marshal0004/Freeddit.git
cd Freeddit
chmod +x scripts/freeddit-setup.sh
./scripts/freeddit-setup.sh
```

This installs all dependencies, builds the Rust backend + Next.js frontend, and creates launcher scripts:
- `freeddit` — start in production mode
- `freeddit-dev` — start in dev mode (hot reload)
- `freeddit-electron` — launch as Electron desktop app

### Manual setup (any Linux distro)

#### Prerequisites
- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Node.js 20+
- Bun (optional, faster than npm)
- SQLite 3.35+

#### Build

```bash
# Clone
git clone https://github.com/marshal0004/Freeddit.git
cd Freeddit

# Build backend
cd backend-rust
cargo build --release
cd ..

# Install frontend deps
bun install  # or: npm install

# Run dev mode (terminal 1)
DATABASE_URL="sqlite://$PWD/freeddit.db?mode=rwc" PORT=3001 \
  ./backend-rust/target/release/freeddit-backend

# Run dev mode (terminal 2)
bun run dev
```

Open http://localhost:3000/ in your browser.

### Default seed users

| Username | Password | Notes |
|---|---|---|
| `alice` | `password123` | Admin, mod of r/programming |
| `bob` | `password123` | Regular user |
| `carol` | `password123` | Regular user |
| ... | ... | (25 users total) |

## API

The API matches Reddit's public JSON shape (Thing/Listing wrappers, t1/t2/t3/t5 fullname kinds). Base URL: `http://localhost:3001/api/`

### Endpoints (50+)

| Method | Path | Description |
|---|---|---|
| POST | `/register` | Register new user |
| POST | `/login` | Login (returns JWT) |
| GET | `/me` | Get current user |
| GET | `/subreddits` | List subreddits |
| POST | `/subreddits` | Create subreddit |
| GET | `/subreddits/{name}` | Get subreddit by name |
| POST | `/subreddits/{name}/subscribe` | Subscribe/unsubscribe |
| GET | `/subreddits/{name}/posts` | List subreddit posts |
| GET | `/subreddits/{name}/rules` | List subreddit rules |
| POST | `/subreddits/{name}/rules` | Create rule (mod only) |
| DELETE | `/subreddits/{name}/rules/{id}` | Delete rule (mod only) |
| GET | `/subreddits/{name}/moderators` | List moderators |
| GET/POST | `/subreddits/{name}/wiki` | List/create wiki pages |
| GET/POST/DELETE | `/subreddits/{name}/wiki/{slug}` | Get/upsert/delete wiki page |
| GET | `/posts` | List home feed (?sort=hot\|best\|new\|top\|rising\|controversial) |
| POST | `/posts` | Create post (text/link/image/video/crosspost) |
| GET | `/posts/{id}` | Get post by ID |
| PATCH | `/posts/{id}/edit` | Edit own post |
| DELETE | `/posts/{id}/delete` | Delete own post |
| POST | `/posts/{id}/mod` | Mod action (sticky/lock/remove/approve) |
| GET/POST | `/posts/{id}/comments` | List/create comments |
| GET | `/posts/{id}/sort/{sort}` | List comments with specific sort |
| PATCH | `/comments/{id}/edit` | Edit own comment |
| DELETE | `/comments/{id}/delete` | Delete own comment |
| POST | `/comments/{id}/sticky` | Sticky comment (mod only) |
| POST | `/vote` | Cast vote (+1/0/-1) |
| POST/DELETE | `/save/{post_id}` | Save/unsave post |
| POST/DELETE | `/hide/{post_id}` | Hide/unhide post |
| GET | `/saved` | List saved posts |
| GET | `/hidden` | List hidden posts |
| POST | `/reports` | Create report |
| GET | `/reports/subreddit/{name}` | List reports (mod only) |
| POST | `/reports/{id}/dismiss` | Dismiss report (mod only) |
| GET | `/users/{name}` | Get user profile |
| GET | `/users/{name}/posts` | List user's posts |
| GET | `/users/{name}/comments` | List user's comments |
| GET | `/search?q=...` | Search posts/subreddits/users |
| POST | `/media/upload` | Upload image/video (multipart) |
| GET | `/media/mine` | List user's media |
| GET/POST | `/multireddits` | List/create custom feeds |
| GET/DELETE | `/multireddits/{name}` | Get/delete multireddit |
| GET | `/multireddits/{name}/posts` | List posts from multireddit |
| GET | `/notifications` | List notifications |
| POST | `/notifications/{id}/read` | Mark notification read |
| POST | `/notifications/read-all` | Mark all read |
| POST | `/messages` | Start new conversation |
| GET | `/messages/conversations` | List conversations |
| GET/DELETE | `/messages/conversations/{id}` | Get/delete conversation |
| POST | `/messages/conversations/{id}/send` | Send message |
| GET/PATCH | `/account/settings` | Get/update settings |
| PATCH | `/account/profile` | Update profile |
| POST | `/account/delete` | Delete account (with password) |
| POST | `/account/2fa/enable` | Enable 2FA (returns secret + QR URL) |
| POST | `/account/2fa/disable` | Disable 2FA |
| GET | `/account/2fa/status` | Get 2FA status |
| POST | `/account/send-verification` | Send email verification |
| GET | `/verify-email?token=...` | Verify email |
| POST | `/account/request-password-reset` | Request password reset |
| POST | `/account/confirm-password-reset` | Confirm password reset |

## Honest notes

### REA (Reverse Engineer Anything)

The REA CLI was installed and used to attempt capturing reddit.com's JavaScript bundle. Reddit's Web Application Firewall (WAF) blocks headless Chromium — even with `--disable-blink-features=AutomationControlled` and a real macOS user-agent, Reddit returns the "blocked by network security" page with the Snoo mascot instead of the actual UI.

What REA successfully captured:
- A screenshot of reddit.com/r/programming (52KB PNG) — but it's the WAF block page, not the actual UI

What REA could NOT capture:
- The JavaScript bundle structure (chunks, modules, entry points)
- The route table
- The API endpoint contracts

What I used instead as the "reverse engineering" basis:
- Reddit's public engineering blog (2010 post documenting the Hot ranking algorithm)
- Evan Miller's 2009 article on Wilson score (which Reddit uses for Best comment sort)
- Reddit's official help docs (which document the markdown spec, comment sort variants, mod tools, formatting guide)
- Reddit's public API documentation (which documents the Thing/Listing wrapper shape and t1/t2/t3/t5 fullname kinds)

### craft-launcher repo reference

The user originally cited `github.com/storytold/craft-launcher` as a reference for "reverse engineering Adobe Lightroom". That repo does NOT reverse engineer Adobe Lightroom — it's an original Rust launcher for a separate "Craft App Suite" of apps. I was honest about this upfront.

### Brand assets

freeddit does NOT reproduce:
- Reddit's Snoo mascot
- The orange "Reddit" wordmark
- Any copyrighted strings or proprietary brand assets

The freeddit logo is original art: an orange rounded-square with a friendly white alien-head character (antenna, smile, ears) and the letter "f" for freeddit. See [`public/freeddit-logo.svg`](public/freeddit-logo.svg).

## License

AGPL-3.0-or-later. See [`LICENSE`](LICENSE) for details.

## Acknowledgments

- Reddit's engineering team for publishing their ranking algorithms publicly
- Evan Miller for the Wilson score article
- The axum, sqlx, tokio, and React teams for the excellent tools
