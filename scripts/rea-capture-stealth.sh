#!/bin/bash
set -uo pipefail
CHROMIUM_PATH="/home/z/.cache/ms-playwright/chromium-1200/chrome-linux64/chrome"
CAPTURE_DIR="/home/z/my-project/rea-captures"
mkdir -p "$CAPTURE_DIR"

# Use stealth flags + realistic user agent
"$CHROMIUM_PATH" \
  --remote-debugging-port=9222 \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --disable-dbus \
  --disable-blink-features=AutomationControlled \
  --user-agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36" \
  --user-data-dir=/tmp/chromium-rea-stealth \
  "https://www.reddit.com/r/programming/hot/" &

CHROME_PID=$!
echo "Chrome PID: $CHROME_PID"

# Wait longer for reddit's WAF to clear
sleep 12

echo ""
echo "=== CDP version ==="
curl -s --max-time 5 http://127.0.0.1:9222/json/version | python3 -c "
import json, sys
d = json.load(sys.stdin)
print('Browser:', d.get('Browser'))
print('UA:', d.get('User-Agent', '?')[:80])
"

# Check what page loaded
TARGETS_JSON=$(curl -s http://127.0.0.1:9222/json)
TARGET_ID=$(echo "$TARGETS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
page = next(t for t in data if t['type'] == 'page')
print(page['id'])
")
TARGET_TITLE=$(echo "$TARGETS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
page = next(t for t in data if t['type'] == 'page')
print(page.get('title', '')[:80])
")
TARGET_URL=$(echo "$TARGETS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
page = next(t for t in data if t['type'] == 'page')
print(page.get('url', '')[:80])
")
echo "Page title: $TARGET_TITLE"
echo "Page URL: $TARGET_URL"
echo "Target ID: $TARGET_ID"

echo ""
echo "=== Capture screenshot ==="
rea capture-web-screenshot "http://127.0.0.1:9222" "$TARGET_ID" --format json > "$CAPTURE_DIR/reddit-stealth-screenshot.json" 2>/dev/null
echo "Screenshot saved"
python3 -c "
import json, base64
d = json.load(open('$CAPTURE_DIR/reddit-stealth-screenshot.json'))
nr = d.get('normalized_result', {})
artifact = nr.get('artifact', {})
if 'data_base64' in artifact:
    png = base64.b64decode(artifact['data_base64'])
    with open('$CAPTURE_DIR/reddit-stealth.png', 'wb') as f:
        f.write(png)
    print(f'PNG saved: {len(png)} bytes')
print(f'Target URL captured: {nr.get(\"target\", {}).get(\"url\", \"?\")}')
print(f'Target title: {nr.get(\"target\", {}).get(\"title\", \"?\")}')
"

echo ""
echo "=== Run analyze-web-bundle (longer observation) ==="
rea analyze-web-bundle "http://127.0.0.1:9222" "$TARGET_ID" --observation-ms 6000 --fetch-source-maps --format json > "$CAPTURE_DIR/reddit-stealth-bundle.json" 2>/dev/null
EXIT_CODE=$?
echo "Bundle exit: $EXIT_CODE, size: $(wc -c < $CAPTURE_DIR/reddit-stealth-bundle.json) bytes"

python3 -c "
import json
d = json.load(open('$CAPTURE_DIR/reddit-stealth-bundle.json'))
nr = d.get('normalized_result', {})
cap = nr.get('capture', {})
print(f'Scripts observed: {cap.get(\"scripts_observed\", 0)}')
print(f'Scripts analyzed: {cap.get(\"scripts_analyzed\", 0)}')
print(f'Source artifacts: {len(cap.get(\"source_artifacts\", []))}')
obs = nr.get('observations', {})
chunks = obs.get('chunks', {})
print(f'Chunk nodes: {len(chunks.get(\"nodes\", []))}')
print(f'Chunk edges: {len(chunks.get(\"edges\", []))}')
print(f'Routes: {len(obs.get(\"routes\", []))}')
print(f'Endpoints: {len(obs.get(\"endpoints\", []))}')
print(f'Source maps items: {len(obs.get(\"source_maps\", {}).get(\"items\", []))}')
"

echo ""
echo "=== Cleanup ==="
kill $CHROME_PID 2>/dev/null
wait $CHROME_PID 2>/dev/null
