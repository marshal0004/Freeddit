#!/bin/bash
#
# rea-reddit-capture.sh — Run REA on Reddit from your Arch Linux machine
#
# This script does what the cloud sandbox COULDN'T do:
# 1. Installs REA + dependencies
# 2. Launches a REAL browser (your actual Chrome/Chromium)
# 3. Navigates to reddit.com (your residential IP won't be WAF blocked)
# 4. Captures ALL JavaScript files Reddit loads
# 5. Runs rea analyze-javascript-application on the extracted JS tree
# 6. Runs rea trace_application_feature on specific features
# 7. Saves everything to ./rea-output/
#
# Usage:
#   chmod +x rea-reddit-capture.sh
#   ./rea-reddit-capture.sh
#
# Requirements:
#   - Arch Linux with internet
#   - Node.js 22.19+ (script will install if missing)
#   - A Chrome/Chromium browser installed
#   - About 5 minutes runtime
#
# OUTPUT:
#   ./rea-output/
#   ├── reddit-screenshot.png       — visual screenshot of reddit.com
#   ├── reddit-dom.html             — full rendered DOM
#   ├── reddit-sw.js                — service worker source
#   ├── reddit-js/                  — all extracted JavaScript files
#   ├── analysis-evidence.json     — REA module graph + semantic analysis
#   ├── feature-traces/             — per-feature evidence (voting, comments, etc.)
#   └── SPEC.md                     — auto-generated spec from evidence

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err() { echo -e "${RED}[error]${NC} $1" >&2; }

OUTPUT_DIR="$HOME/rea-output"
mkdir -p "$OUTPUTDIR"

# === Step 0: Check prerequisites ===
log "Step 0: Checking prerequisites..."

# Check Node.js
if ! command -v node &>/dev/null; then
    warn "Node.js not found. Installing via nvm..."
    curl -sSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
    export NVM_DIR="$HOME/.nvm"
    [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
    nvm install 24
fi

NODE_VERSION=$(node -v | sed 's/v//' | cut -d. -f1)
if [ "$NODE_VERSION" -lt 22 ]; then
    err "Node.js $NODE_VERSION is too old. Need v22.19+. Run: nvm install 24"
    exit 1
fi
log "  Node.js: $(node -v) ✓"

# Check npm
if ! command -v npm &>/dev/null; then
    err "npm not found. Install Node.js first."
    exit 1
fi
log "  npm: $(npm -v) ✓"

# Check for Chrome/Chromium
CHROME=""
for path in \
    "/usr/bin/google-chrome-stable" \
    "/usr/bin/google-chrome" \
    "/usr/bin/chromium" \
    "/usr/bin/chromium-browser" \
    "/usr/bin/brave-browser" \
    "$HOME/.cache/ms-playwright/chromium-*/chrome-linux64/chrome"; do
    if [ -f "$path" ] || ls $path &>/dev/null; then
        CHROME=$(ls $path 2>/dev/null | head -1)
        break
    fi
done

if [ -z "$CHROME" ]; then
    warn "Chrome/Chromium not found. Install one:"
    echo "  sudo pacman -S chromium        # OR"
    echo "  sudo pacman -S google-chrome   # (from AUR)"
    echo ""
    echo "Or install Playwright's Chromium:"
    echo "  npx playwright install chromium"
    read -p "Install Playwright Chromium now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        npx playwright install chromium
        CHROME=$(find ~/.cache/ms-playwright -name 'chrome' -type f 2>/dev/null | head -1)
    else
        err "A Chrome-family browser is required. Install one and re-run."
        exit 1
    fi
fi
log "  Browser: $CHROME ✓"

# === Step 1: Install REA ===
log ""
log "Step 1: Installing REA..."
if ! command -v rea &>/dev/null; then
    npm install -g rea-agents
fi
log "  REA version: $(rea --version) ✓"

# === Step 2: Capture Reddit with capture_browser_scenario ===
log ""
log "Step 2: Capturing reddit.com with REA capture_browser_scenario..."
log "  (Your residential IP should NOT be WAF-blocked)"

# Create the scenario JSON
cat > "$OUTPUT_DIR/scenario.json" <<SCENARIO
{
  "browser": {
    "mode": "launch",
    "executable_path": "$CHROME",
    "headless": false
  },
  "start_url": { "url": "https://www.reddit.com/r/programming/" },
  "actions": [
    {
      "step_id": "wait_for_page_load",
      "action": "wait_for_timeout",
      "duration_ms": 10000
    },
    {
      "step_id": "scroll_to_load_more",
      "action": "press",
      "key": "End",
      "locator": { "selector": "body" }
    },
    {
      "step_id": "wait_after_scroll",
      "action": "wait_for_timeout",
      "duration_ms": 5000
    }
  ],
  "capture": {
    "after_each_step": ["screenshot", "dom", "accessibility", "url", "history", "storage"],
    "at_end": ["screenshot", "dom", "accessibility", "url", "history", "storage"],
    "events": ["console", "page-errors", "network", "websockets", "frames", "workers", "popups", "downloads"],
    "network": {
      "request_body": true,
      "response_body": true,
      "header_values": true
    }
  }
}
SCENARIO

log "  Running capture_browser_scenario..."
rea capture-browser-scenario "$OUTPUT_DIR/scenario.json" --format json > "$OUTPUT_DIR/capture-result.json" 2>&1
CAPTURE_EXIT=$?

if [ $CAPTURE_EXIT -eq 0 ]; then
    log "  Capture successful! ✓"
    log "  Evidence saved to: $OUTPUT_DIR/capture-result.json"
    
    # Extract screenshot and DOM
    python3 <<'PYEOF'
import json, base64, os
d = json.load(open(os.path.expanduser('~/rea-output/capture-result.json')))
nr = d.get('normalized_result', {})
for step in nr.get('steps', []):
    artifacts = step.get('artifacts', {})
    for k, v in (artifacts or {}).items():
        if isinstance(v, dict) and v.get('state') == 'captured':
            val = v.get('value', {})
            if isinstance(val, dict):
                if k == 'screenshot' and 'data_base64' in val:
                    png = base64.b64decode(val['data_base64'])
                    with open(os.path.expanduser('~/rea-output/reddit-screenshot.png'), 'wb') as f:
                        f.write(png)
                    print(f'  Screenshot saved: {len(png)} bytes')
                elif k == 'dom' and 'text' in val:
                    with open(os.path.expanduser('~/rea-output/reddit-dom.html'), 'w') as f:
                        f.write(val['text'])
                    print(f'  DOM saved: {len(val["text"])} bytes')
    
# Count network events
events = nr.get('events', {})
items = events.get('items', [])
print(f'  Network events: {len(items)}')
scripts = [i for i in items if isinstance(i, dict) and 'script' in i.get('kind', '').lower()]
network = [i for i in items if isinstance(i, dict) and 'network' in i.get('kind', '').lower()]
print(f'  Script events: {len(scripts)}')
print(f'  Network events: {len(network)}')

# Extract JS URLs from network events
js_urls = set()
for item in items:
    if isinstance(item, dict):
        url = item.get('url', {})
        if isinstance(url, dict):
            u = url.get('url', '')
        elif isinstance(url, str):
            u = url
        else:
            u = ''
        if u and ('.js' in u or 'javascript' in str(item.get('mime_type', ''))):
            js_urls.add(u)

if js_urls:
    print(f'\n  JavaScript URLs found: {len(js_urls)}')
    with open(os.path.expanduser('~/rea-output/js-urls.txt'), 'w') as f:
        for u in sorted(js_urls):
            f.write(u + '\n')
            print(f'    {u[:100]}')
PYEOF
else
    warn "Capture returned exit code $CAPTURE_EXIT"
    cat "$OUTPUT_DIR/capture-result.json" | head -20
fi

# === Step 3: Download Reddit's service worker (always works) ===
log ""
log "Step 3: Downloading Reddit's service worker..."
curl -sSL -A "Mozilla/5.0" "https://www.reddit.com/sw.js" -o "$OUTPUT_DIR/reddit-sw.js" --max-time 15
log "  Service worker: $(wc -c < "$OUTPUT_DIR/reddit-sw.js") bytes ✓"

# Download robots.txt
curl -sSL -A "Mozilla/5.0" "https://www.reddit.com/robots.txt" -o "$OUTPUT_DIR/reddit-robots.txt" --max-time 10
log "  robots.txt: $(wc -c < "$OUTPUT_DIR/reddit-robots.txt") bytes ✓"

# === Step 4: Download all JavaScript files from the captured URLs ===
log ""
log "Step 4: Downloading Reddit's JavaScript files..."

JS_DIR="$OUTPUT_DIR/reddit-js"
mkdir -p "$JS_DIR"

if [ -f "$OUTPUT_DIR/js-urls.txt" ]; then
    COUNT=0
    TOTAL=$(wc -l < "$OUTPUT_DIR/js-urls.txt")
    
    while IFS= read -r url; do
        COUNT=$((COUNT + 1))
        # Create a safe filename from the URL
        FILENAME=$(echo "$url" | sed 's|https\?://||; s|/|_|g; s|\?|_|g' | head -c 200)
        
        echo -ne "  [$COUNT/$TOTAL] Downloading: ${url:0:80}..."
        
        HTTP_CODE=$(curl -s -o "$JS_DIR/$FILENAME" -w "%{http_code}" \
            -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36" \
            -H "Accept: */*" \
            -H "Referer: https://www.reddit.com/" \
            --max-time 30 \
            "$url" 2>/dev/null)
        
        SIZE=$(wc -c < "$JS_DIR/$FILENAME" 2>/dev/null || echo 0)
        
        if [ "$HTTP_CODE" = "200" ] && [ "$SIZE" -gt 100 ]; then
            echo -e " ${GREEN}✓ (${SIZE} bytes)${NC}"
        else
            echo -e " ${RED}✗ (HTTP $HTTP_CODE, ${SIZE} bytes)${NC}"
            rm -f "$JS_DIR/$FILENAME"
        fi
    done < "$OUTPUT_DIR/js-urls.txt"
    
    log "  Downloaded $(ls "$JS_DIR"/*.js 2>/dev/null | wc -l) JS files"
else
    warn "No JS URLs found in capture result."
    warn "If Reddit's WAF blocked you, try these alternatives:"
    echo "  1. Use a VPN to a US residential IP"
    echo "  2. Log in to Reddit first (the WAF may allow authenticated users)"
    echo "  3. Try old.reddit.com (sometimes less protected)"
fi

# === Step 5: Run rea analyze-javascript-application on the JS tree ===
log ""
log "Step 5: Running REA analyze-javascript-application on extracted JS..."

if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    log "  Analyzing $(ls "$JS_DIR" | wc -l) files..."
    
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUTPUT_DIR/analysis-evidence.json" 2>&1
    ANALYSIS_EXIT=$?
    
    if [ $ANALYSIS_EXIT -eq 0 ]; then
        log "  Analysis successful! ✓"
        log "  Evidence saved to: $OUTPUT_DIR/analysis-evidence.json"
        
        # Extract summary
        python3 <<'PYEOF'
import json, os
d = json.load(open(os.path.expanduser('~/rea-output/analysis-evidence.json')))
nr = d.get('normalized_result', {})
print(f'  Evidence ID: {d.get("evidence_id", "?")[:50]}...')
print(f'  Graph nodes: {len(nr.get("graph", {}).get("nodes", []))}')
print(f'  Graph edges: {len(nr.get("graph", {}).get("edges", []))}')
print(f'  Semantic nodes: {len(nr.get("semantic_graph", {}).get("nodes", []))}')
print(f'  Statistics: {nr.get("statistics", {})}')
PYEOF
    else
        warn "Analysis returned exit code $ANALYSIS_EXIT"
        head -5 "$OUTPUT_DIR/analysis-evidence.json"
    fi
else
    warn "No JS files to analyze (download may have failed)"
fi

# === Step 6: Also try rea export-web-scripts ===
log ""
log "Step 6: Running REA export-web-scripts..."

EXPORT_DIR="$OUTPUT_DIR/exported-scripts"
mkdir -p "$EXPORT_DIR"

if [ -f "$OUTPUT_DIR/capture-result.json" ]; then
    rea export-web-scripts "$OUTPUT_DIR/capture-result.json" "$EXPORT_DIR" --format json 2>&1 | tail -5
    log "  Exported scripts: $(ls "$EXPORT_DIR" 2>/dev/null | wc -l) files"
fi

# === Step 7: Summary ===
log ""
log "=== CAPTURE COMPLETE ==="
echo ""
echo "Output directory: $OUTPUT_DIR/"
echo ""
echo "Files produced:"
ls -lh "$OUTPUT_DIR/" 2>/dev/null
echo ""
if [ -d "$JS_DIR" ]; then
    echo "JavaScript files:"
    ls -lh "$JS_DIR/" 2>/dev/null | head -20
    echo ""
    echo "Total JS files: $(ls "$JS_DIR" 2>/dev/null | wc -l)"
    echo "Total JS size: $(du -sh "$JS_DIR" 2>/dev/null | awk '{print $1}')"
fi
echo ""
if [ -f "$OUTPUT_DIR/analysis-evidence.json" ]; then
    echo "Analysis evidence: $(ls -lh "$OUTPUT_DIR/analysis-evidence.json" | awk '{print $5}')"
fi
echo ""
echo -e "${BLUE}NEXT STEPS:${NC}"
echo "  1. Review the captured screenshot: xdg-open $OUTPUT_DIR/reddit-screenshot.png"
echo "  2. Review the DOM: less $OUTPUT_DIR/reddit-dom.html"
echo "  3. Review the service worker: less $OUTPUT_DIR/reddit-sw.js"
echo "  4. If JS was captured, analyze it:"
echo "     rea analyze-javascript-application $OUTPUT_DIR/reddit-js --format json > $OUTPUT_DIR/full-analysis.json"
echo "  5. Trace specific features:"
echo "     rea trace-application-feature $OUTPUT_DIR/full-analysis.json --feature voting"
echo ""
echo -e "${YELLOW}If Reddit's WAF blocked you (HTTP 403):${NC}"
echo "  - You may be on a data center / VPN IP"
echo "  - Try: curl -s -o /dev/null -w '%{http_code}' https://www.reddit.com/"
echo "  - If it returns 403, use a residential proxy or VPN"
echo "  - If it returns 200, you're not blocked — re-run this script"
echo ""
echo "Share your results in the REA Discord: https://discord.gg/reverse-engineering"
