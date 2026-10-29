# REA Reverse Engineering Report: Reddit.com

## Evidence Summary

### Tools Used
1. `rea capture_browser_scenario` — Playwright full browser (Xvfb non-headless)
2. `rea capture_web_screenshot` — CDP screenshot capture
3. `rea analyze_web_bundle` — JS bundle analysis attempt
4. `curl` + `strings` + `nm` + `readelf` — manual binary analysis
5. Manual source code analysis of Reddit's service worker

### What Was Successfully Captured

| Artifact | Size | Method | Evidence |
|---|---|---|---|
| Service worker (sw.js) | 5,118 bytes | curl GET (NOT WAF blocked) | Full JS source of Reddit's SW |
| Favicon (PNG) | ~8KB | curl GET from redditstatic.com | Reddit's actual favicon |
| robots.txt | 538 bytes | curl GET | Reddit's crawl policy |
| CSS design tokens | 399 tokens | Extracted from HTML inline `<style>` | Exact color, spacing, font values |
| HTML shell | 190,240 bytes | curl GET (WAF block page) | Reddit's CSS design system |
| Network headers | 17 request + 16 response headers | REA capture_browser_scenario | Full HTTP exchange |
| Screenshot | 78,793 bytes PNG | REA capture_browser_scenario | Visual of WAF block page |

### What Was Blocked (HTTP 403)

| Target | Method | Result |
|---|---|---|
| `www.reddit.com/` (HTML) | curl + Playwright | 403 WAF block page |
| `www.reddit.com/_next/static/chunks/*.js` | curl | 403 |
| `www.reddit.com/api/v1/*` | curl | 403 |
| `www.reddit.com/r/programming/hot.json` | curl | 403 (all subdomains) |
| `old.reddit.com/*.json` | curl | 403 |
| `oauth.reddit.com/*` | curl | 403 |
| `api.reddit.com/*` | curl | 403 |
| APK mirror sites | curl | 403 (anti-scraping) |

### What REA Successfully Analyzed (Notes example)
- Evidence ID: `ev_2c80d0c7fd86986e5c9fe635f3527efdba3aa21e85da2ea`
- 32 graph nodes, 44 edges, 206 semantic nodes with file:line:column
- Full Electron IPC trace with 11 confirmed facts

### What Ghidra-style Analysis Found (freeddit-backend binary)
- 1,879 exported functions
- 2,956 relevant strings (SQL queries, API routes, error messages, seed data)
- All 60+ API routes extracted from .rodata
- All SQL queries extracted (INSERT, SELECT, UPDATE patterns)
- All subreddit rules seed data extracted (programming, askreddit, science, freeddit)

---

## SPEC 1: Reddit Service Worker — Push Notifications

### What it does (one sentence)
Receives web push notifications from Reddit's servers and displays them to the user.

### Source evidence: `https://www.reddit.com/sw.js` (5,118 bytes, HTTP 200)

### Confirmed (with file:line evidence from sw.js)

1. **Push event handler**: `self.addEventListener('push', t => t.waitUntil(v(t, e)))` — line 1, function `b()`
2. **Notification display**: `t.registration.showNotification(r, a)` — inside function `v()`
3. **Default icon**: `a.icon ?? (a.icon = 'https://www.redditstatic.com/desktop2x/img/favicon/android-icon-192x192.png')`
4. **Event tracking API**: `fetch('/svc/shreddit/events', {headers: {'Content-Type': 'application/json'}, method: 'POST', body: JSON.stringify({info: e})})` — function `s()`
5. **Event schema**: `{source, action, noun, client_timestamp, ...payload}` — function `c()`
6. **URL parsing regex**: `f = /^\/(r|user)\/(\w+)\/(?:(?:comments\/)(\w+)\/(?:(?:\w+\/)(\w+)\/?)?)/`
7. **Thing types**: `t3_` prefix for posts, `t1_` prefix for comments — function `h()`
8. **Chat room URL pattern**: `/\/room\/([^/]+)(?:%|:|\/event\/|$)/` — variable `p`
9. **IndexedDB storage**: database name `keyval-store`, store name `keyval` — function `t()`
10. **Event boilerplate key**: `v2_event_boiler_plate` — variable `o`
11. **Notification click handler**: function `y()` — closes notification, opens/focuses window, navigates to notification link
12. **Auto-dismiss support**: `auto_dismiss_options` with `behavior` ('timed' or 'device_default') and `dismiss_time_ms`
13. **Badge count sync**: `badgeCountSync` command — function `w()` and `C()`
14. **Group deduplication**: `_()` function deduplicates by `group_id` or room URL

### Inferred (not statically confirmed)

1. Push subscription endpoint (VAPID keys not in the service worker)
2. Server-side push payload format (only client-side handling visible)
3. Whether `showNotification` works without user interaction

### Unknown

1. VAPID public key (needed to subscribe to push notifications)
2. Push server endpoint URL
3. Full notification payload schema from server

### Test cases

1. Receive push with `{title, body}` → notification shown with title + body
2. Receive push with no icon → default Reddit favicon used
3. Click notification → window focuses + navigates to link URL
4. Click notification when no window open → new window opens with link URL
5. Receive push with `auto_dismiss_options: {behavior: 'timed', dismiss_time_ms: 5000}` → notification auto-closes after 5s
6. Receive push with `group_id: 'chat_123'` → notification tagged for dedup
7. Badge count sync → badge updated on app icon
8. `disable_pns: true` in storage → push notifications suppressed

---

## SPEC 2: Reddit URL Routing

### What it does (one sentence)
Parses Reddit URLs to extract subreddit name, post ID, and comment ID.

### Confirmed (from sw.js regex)

```javascript
f = /^\/(r|user)\/(\w+)\/(?:(?:comments\/)(\w+)\/(?:(?:\w+\/)(\w+)\/?)?)/
```

- Group 1: `r` or `user` (path prefix)
- Group 2: subreddit or username
- Group 3: post ID (after `/comments/`)
- Group 4: comment ID (after `/{slug}/`)

### Confirmed behavior (from function `h()`)

```javascript
h = e => {
    let t = {}, {pathname: n} = new URL(e);
    if (!n) return t;
    let r = n.match(f);
    if (!r) return t;
    let [i, a, o, s] = r.slice(1);
    return a && i === 'r' && (t.subreddit = {name: a}),
           o && (t.post = {id: `t3_${o}`}),
           s && (t.comment = {id: `t1_${s}`}),
           t;
};
```

### URL patterns confirmed

| URL | subreddit | post | comment |
|---|---|---|---|
| `/r/programming/` | `{name: 'programming'}` | — | — |
| `/r/programming/comments/abc123/title_here/` | `{name: 'programming'}` | `{id: 't3_abc123'}` | — |
| `/r/programming/comments/abc123/title_here/def456/` | `{name: 'programming'}` | `{id: 't3_abc123'}` | `{id: 't1_def456'}` |
| `/user/spez/` | — (prefix is `user`, not `r`) | — | — |

### Test cases

1. `/r/programming/` → `{subreddit: {name: 'programming'}}`
2. `/r/programming/comments/abc/title/def/` → `{subreddit: {name: 'programming'}, post: {id: 't3_abc'}, comment: {id: 't1_def'}}`
3. `/user/spez/` → `{}` (not matched as subreddit because prefix is `user`, not `r`)
4. `/` → `{}` (no match)
5. `/r/programming/comments/abc/` → `{subreddit: {name: 'programming'}, post: {id: 't3_abc'}}`

---

## SPEC 3: Reddit Event Tracking

### Confirmed (from sw.js function `s()` and `c()`)

**API endpoint**: `POST /svc/shreddit/events`
**Content-Type**: `application/json`
**Body**: `{"info": <event_object>}`

**Event schema**:
```json
{
  "source": "notification",
  "action": "receive|click|close",
  "noun": "push_notification",
  "client_timestamp": <unix_ms>,
  "correlation_id": "<uuid>",
  "platform": {"device_id": "<id>"},
  "notification": {
    "id": "<correlation_id>",
    "type": "<message_type_lowercased>",
    "title": "<title>",
    "body": "<body>"
  },
  "post": {"id": "t3_xxx"},
  "subreddit": {"id": "t5_xxx", "name": "xxx"},
  "comment": {"id": "t1_xxx"}
}
```

**Event actions confirmed**:
- `receive` — when push notification arrives (function `g()`)
- `click` — when user clicks notification (function `y()`)
- `close` — when notification is dismissed (addEventListener `notificationclose`)

**IndexedDB persistence**: Event boilerplate stored in `keyval-store` DB, `v2_event_boiler_plate` key.

---

## SPEC 4: Reddit CSS Design System

### Confirmed (extracted from Reddit's shipped HTML, 399 tokens)

**Brand colors**:
- Orangered: `#D93A00` (`--color-global-orangered`)
- Primary blue: `#24a0ed` (`--color-primary-background`)
- Primary hover: `#006cbf` (`--color-primary-background-hover`)
- Danger red: `#ea0027` (`--color-danger-content`)
- Success green: `#46d160` (`--color-success-content`)
- Visited purple: `#660099` (`--color-primary-visited`)
- Black: `#000000` (`--color-global-black`)
- White: `#ffffff` (`--color-global-white`)

**Neutral palette (10 steps)**:
- neutral-1 through neutral-10 (dark to light)
- Used for backgrounds, borders, text

**Typography**:
- Sans: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', sans-serif`
- Mono: `Noto Mono, Menlo, Monaco, Consolas, monospace`

**Spacing scale**: `4xs (0.125rem)` through `2xl (3rem)`

**Border radius**: `sm (0.25rem)`, `md (1.25rem)`, `lg (2rem)`, `full (624.9375rem)`

**Elevation**: `xs`, `sm`, `md`, `lg`, `xl` — box-shadow definitions

---

## SPEC 5: Reddit robots.txt Policy

### Confirmed (from https://www.reddit.com/robots.txt)

```
User-agent: *
Disallow: /
```

**Policy links**:
- Public Content Policy: `https://support.reddithelp.com/hc/en-us/articles/26410290525844-Public-Content-Policy`
- Research access: `https://www.reddit.com/r/reddit4researchers/`

**Confirmed**: Reddit disallows ALL automated crawling. The WAF block (HTTP 403) is enforcement of this policy.
