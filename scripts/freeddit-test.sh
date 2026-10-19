#!/bin/bash
# Self-contained freeddit test script.
# Starts backend + dev server in the SAME bash session (as children),
# runs all tests, and reports results.

set -e
cd /home/z/my-project

echo "==================================================="
echo "[$(date)] Starting freeddit test run"
echo "==================================================="

# Cleanup any leftover processes
pkill -9 -f "freeddit-backend" 2>/dev/null || true
pkill -9 -f "next dev" 2>/dev/null || true
pkill -9 -f "next-server" 2>/dev/null || true
sleep 2

# Start backend as a child of this bash session
DATABASE_URL="sqlite:///home/z/my-project/freeddit.db" PORT=3001 \
  /home/z/my-project/backend-rust/target/release/freeddit-backend \
  > /home/z/my-project/backend.log 2>&1 &
BACKEND_PID=$!
echo "Backend PID: $BACKEND_PID"

# Start Next.js dev server as a child of this bash session
bun run dev > /home/z/my-project/dev.log 2>&1 &
DEV_PID=$!
echo "Dev server PID: $DEV_PID"

# Wait for both to be ready
echo "Waiting for services to be ready..."
for i in {1..30}; do
  if curl -s --max-time 2 http://localhost:3001/api/health 2>/dev/null | grep -q "ok"; then
    echo "  Backend ready (after ${i}s)"
    break
  fi
  sleep 1
done

for i in {1..30}; do
  if curl -s --max-time 2 -o /dev/null -w "%{http_code}" http://localhost:3000/ 2>/dev/null | grep -q "200"; then
    echo "  Dev server ready (after ${i}s)"
    break
  fi
  sleep 1
done

# Final readiness check
echo ""
echo "=== Final health check ==="
echo "Backend: $(curl -s http://localhost:3001/api/health 2>&1)"
curl -s -o /tmp/page.html -w "Next.js direct: status=%{http_code} size=%{size_download}\n" http://localhost:3000/ --max-time 30
echo "Contains 'freeddit': $(grep -c freeddit /tmp/page.html 2>/dev/null || echo 0)"
curl -s -o /tmp/caddy.html -w "Caddy: status=%{http_code} size=%{size_download}\n" http://localhost:81/ --max-time 30
echo "Contains 'freeddit' via Caddy: $(grep -c freeddit /tmp/caddy.html 2>/dev/null || echo 0)"

echo ""
echo "=== API endpoint tests ==="
echo "--- /api/posts?sort=hot ---"
curl -s "http://localhost:81/api/posts?XTransformPort=3001&sort=hot&limit=3" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print('Posts returned:', len(d['data']['children']))
for p in d['data']['children']:
    pd = p['data']
    print(f'  - {pd[\"title\"][:55]} | r/{pd[\"subreddit\"]} | score={pd[\"score\"]} | comments={pd[\"num_comments\"]}')
" 2>&1

echo ""
echo "--- /api/subreddits/popular ---"
curl -s "http://localhost:81/api/subreddits/popular?XTransformPort=3001&limit=5" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print('Subreddits:', len(d['data']['children']))
for s in d['data']['children']:
    sd = s['data']
    print(f'  - r/{sd[\"display_name\"]} | subs={sd[\"subscribers\"]}')
" 2>&1

echo ""
echo "--- /api/posts (sort variations) ---"
for sort in hot best new top rising controversial; do
  curl -s "http://localhost:81/api/posts?XTransformPort=3001&sort=$sort&limit=1" | python3 -c "
import json, sys
d = json.load(sys.stdin)
if d.get('data',{}).get('children'):
  p = d['data']['children'][0]['data']
  print(f'  $sort: {p[\"title\"][:40]} score={p[\"score\"]}')
else:
  print(f'  $sort: (no posts)')
" 2>&1
done

echo ""
echo "--- /api/register + /api/login + /api/vote ---"
TOKEN=$(curl -s -X POST "http://localhost:81/api/register?XTransformPort=3001" \
  -H "Content-Type: application/json" \
  -d '{"username":"testq1","password":"test123"}' | python3 -c "import json,sys; print(json.load(sys.stdin).get('access_token',''))")
echo "Token: ${TOKEN:0:25}..."

FIRST_POST=$(curl -s "http://localhost:81/api/posts?XTransformPort=3001&limit=1" | python3 -c "import json,sys; print(json.load(sys.stdin)['data']['children'][0]['data']['name'])")
echo "Voting on: $FIRST_POST"

curl -s -X POST "http://localhost:81/api/vote?XTransformPort=3001" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"target_id\":\"$FIRST_POST\",\"direction\":1}" | python3 -c "
import json,sys
d = json.load(sys.stdin)
print(f'Vote: changed={d[\"changed\"]} score={d[\"score\"]} likes={d[\"likes\"]}')
" 2>&1

echo ""
echo "--- /api/subreddits/:name ---"
curl -s "http://localhost:81/api/subreddits/programming?XTransformPort=3001" | python3 -c "
import json,sys
d = json.load(sys.stdin)
print(f'r/{d[\"data\"][\"display_name\"]} | subs={d[\"data\"][\"subscribers\"]} | title={d[\"data\"][\"title\"]}')
" 2>&1

echo ""
echo "--- /api/posts/:id/comments ---"
curl -s "http://localhost:81/api/posts/$FIRST_POST/comments?XTransformPort=3001&sort=best" | python3 -c "
import json,sys
d = json.load(sys.stdin)
print(f'Top-level comments: {len(d[\"data\"][\"children\"])}')
for c in d['data']['children'][:2]:
    cd = c['data']
    print(f'  - score={cd[\"score\"]} replies={len(cd.get(\"replies\",[]))} body: {cd[\"body\"][:40]}')
" 2>&1

echo ""
echo "--- /api/search?q=rust ---"
curl -s "http://localhost:81/api/search?XTransformPort=3001&q=rust" | python3 -c "
import json,sys
d = json.load(sys.stdin)
print(f'Search results: {len(d[\"data\"][\"children\"])}')
for r in d['data']['children'][:3]:
    title = r['data'].get('title', r['data'].get('display_name', r['data'].get('name', '')))
    print(f'  - [{r[\"kind\"]}] {title[:50]}')
" 2>&1

echo ""
echo "==================================================="
echo "[$(date)] All API tests complete"
echo "==================================================="

# Now run agent-browser tests
echo ""
echo "=== Browser test ==="
agent-browser open "http://localhost:81/?nocache=$(date +%s%N)" 2>&1 | tail -3
sleep 8
agent-browser wait --load networkidle 2>&1 | tail -3
echo ""
echo "--- Snapshot interactive ---"
agent-browser snapshot -i 2>&1 | head -30
echo ""
echo "--- Page title ---"
agent-browser get title 2>&1 | tail -3
echo ""
echo "--- Screenshot ---"
agent-browser screenshot /home/z/my-project/download/freeddit-home.png 2>&1 | tail -3
echo ""
echo "--- Network requests ---"
agent-browser network requests --filter api 2>&1 | head -15

echo ""
echo "=== Try clicking 'Best' sort ==="
agent-browser click @e28 2>&1 | tail -3 || echo "(couldn't click @e28 — re-snapshot)"
sleep 3
agent-browser snapshot -i 2>&1 | head -20

echo ""
echo "=== Try clicking first post ==="
agent-browser snapshot -i 2>&1 | head -30

# Cleanup
echo ""
echo "=== Cleanup ==="
kill $BACKEND_PID 2>/dev/null || true
kill $DEV_PID 2>/dev/null || true
echo "Done"
