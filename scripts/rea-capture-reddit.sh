#!/bin/bash
# Single-shot: launch chrome + run all REA captures + extract screenshots/scripts.
# All within ONE bash session so chrome doesn't get killed by parent process exit.

set -uo pipefail
CHROMIUM_PATH="/home/z/.cache/ms-playwright/chromium-1200/chrome-linux64/chrome"
CAPTURE_DIR="/home/z/my-project/rea-captures"
mkdir -p "$CAPTURE_DIR"

echo "=== Launching Chromium ==="
"$CHROMIUM_PATH" \
  --remote-debugging-port=9222 \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --disable-dbus \
  --user-data-dir=/tmp/chromium-rea-final \
  "https://www.reddit.com/" &
CHROME_PID=$!
echo "Chrome PID: $CHROME_PID"

# Wait for chrome to fully load + reddit to render
echo "Waiting for reddit.com to load..."
for i in {1..30}; do
  if curl -s --max-time 1 http://127.0.0.1:9222/json/version 2>/dev/null | grep -q Browser; then
    echo "  Chrome ready (${i}s)"
    break
  fi
  sleep 1
done

# Give reddit more time to render
sleep 8

echo ""
echo "=== Capture targets ==="
TARGETS_JSON=$(curl -s http://127.0.0.1:9222/json)
echo "$TARGETS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for t in data:
    if t['type'] == 'page':
        print(f'Page: {t[\"url\"][:80]}')
        print(f'Title: {t[\"title\"]}')
        print(f'ID: {t[\"id\"]}')
"

# Get the page target id
TARGET_ID=$(echo "$TARGETS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
page = next(t for t in data if t['type'] == 'page')
print(page['id'])
")
echo ""
echo "Page target ID: $TARGET_ID"

echo ""
echo "=== Run rea capture-web-screenshot ==="
rea capture-web-screenshot "http://127.0.0.1:9222" "$TARGET_ID" --format json > "$CAPTURE_DIR/reddit-home-screenshot.json" 2>/dev/null
echo "Screenshot saved: $(ls -la $CAPTURE_DIR/reddit-home-screenshot.json | awk '{print $5}') bytes"

# Extract PNG
python3 -c "
import json, base64
d = json.load(open('$CAPTURE_DIR/reddit-home-screenshot.json'))
nr = d.get('normalized_result', {})
artifact = nr.get('artifact', {})
if 'data_base64' in artifact:
    png = base64.b64decode(artifact['data_base64'])
    with open('$CAPTURE_DIR/reddit-home.png', 'wb') as f:
        f.write(png)
    print(f'Extracted PNG: {len(png)} bytes')
    print(f'SHA256: {artifact.get(\"sha256\")}')
"

echo ""
echo "=== Run rea analyze-web-bundle ==="
rea analyze-web-bundle "http://127.0.0.1:9222" "$TARGET_ID" --observation-ms 4000 --format json > "$CAPTURE_DIR/reddit-bundle.json" 2>/dev/null
EXIT_CODE=$?
echo "Bundle exit code: $EXIT_CODE"
echo "Bundle saved: $(ls -la $CAPTURE_DIR/reddit-bundle.json | awk '{print $5}') bytes"

python3 -c "
import json
try:
    d = json.load(open('$CAPTURE_DIR/reddit-bundle.json'))
    print('Top keys:', list(d.keys())[:8])
    if 'code' in d:
        print('Error code:', d.get('code'))
        print('Message:', d.get('message', '')[:150])
    else:
        nr = d.get('normalized_result', {})
        print('Result keys:', list(nr.keys())[:15] if isinstance(nr, dict) else type(nr))
        # Save script list if available
        if isinstance(nr, dict):
            scripts = nr.get('scripts', []) or nr.get('captured_scripts', []) or []
            print(f'Scripts captured: {len(scripts)}')
            if scripts:
                with open('$CAPTURE_DIR/reddit-scripts.json', 'w') as f:
                    json.dump(scripts, f, indent=2)
                print(f'Saved script list to reddit-scripts.json')
                print('Sample script:', list(scripts[0].keys()) if isinstance(scripts[0], dict) else scripts[0])
except Exception as e:
    print('Parse error:', e)
"

echo ""
echo "=== Cleanup chrome ==="
kill $CHROME_PID 2>/dev/null
wait $CHROME_PID 2>/dev/null
echo "Done"
