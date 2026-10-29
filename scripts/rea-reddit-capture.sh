#!/bin/bash
#
# rea-reddit-capture.sh — Run REA on Reddit from your Arch Linux machine
#
# Your residential IP is NOT WAF-blocked (confirmed: curl returns 200)
#
# Usage:
#   chmod +x scripts/rea-reddit-capture.sh
#   ./scripts/rea-reddit-capture.sh
#
# Output goes to ~/rea-output/

set -uo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log()  { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err()  { echo -e "${RED}[error]${NC} $1" >&2; }

OUTPUT_DIR="$HOME/rea-output"
mkdir -p "$OUTPUT_DIR"

# === Step 0: Check prerequisites ===
log "Step 0: Checking prerequisites..."

if ! command -v node &>/dev/null; then
    err "Node.js not found. Install: sudo pacman -S nodejs npm"
    exit 1
fi
log "  Node.js: $(node -v) ✓"
log "  npm: $(npm -v) ✓"

# Find Chrome
CHROME=""
for path in \
    "/usr/bin/google-chrome-stable" \
    "/usr/bin/google-chrome" \
    "/usr/bin/chromium" \
    "/usr/bin/chromium-browser" \
    "/usr/bin/brave-browser"; do
    if [ -x "$path" ]; then
        CHROME="$path"
        break
    fi
done

if [ -z "$CHROME" ]; then
    err "Chrome/Chromium not found. Install: sudo pacman -S chromium"
    exit 1
fi
log "  Browser: $CHROME ✓"

# === Step 1: Install REA ===
log ""
log "Step 1: Installing REA..."
if ! command -v rea &>/dev/null; then
    npm install -g rea-agents
fi
log "  REA version: $(rea --version) ✓"

# === Step 2: Download Reddit's HTML (find JS chunk URLs) ===
log ""
log "Step 2: Downloading Reddit HTML to find JS chunk URLs..."
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"
curl -sSL -A "$UA" "https://www.reddit.com/r/programming/" -o "$OUTPUT_DIR/reddit-home.html" --max-time 30
HTML_SIZE=$(wc -c < "$OUTPUT_DIR/reddit-home.html")
log "  HTML: $HTML_SIZE bytes"

if [ "$HTML_SIZE" -lt 10000 ]; then
    err "Reddit HTML too small ($HTML_SIZE bytes) — you may be WAF-blocked"
    err "Check: curl -s -o /dev/null -w '%{http_code}' https://www.reddit.com/"
    exit 1
fi

# Check if it's the WAF block page
if grep -q "blocked by network security" "$OUTPUT_DIR/reddit-home.html"; then
    err "WAF block page detected! Your IP is blocked by Reddit."
    err "Try: different network, disable VPN, or use mobile hotspot"
    exit 1
fi
log "  Not WAF blocked! ✓"

# === Step 3: Extract JS URLs from HTML ===
log ""
log "Step 3: Extracting JavaScript URLs from Reddit HTML..."

# Reddit uses Next.js — JS files are in <script> tags with src or in inline RSC data
grep -oP 'src="[^"]*\.js[^"]*"' "$OUTPUT_DIR/reddit-home.html" | sed 's/src="//;s/"//' > "$OUTPUT_DIR/js-urls-raw.txt"
# Also check for _next/static references
grep -oP 'https?://[^"'\''>\s]+\.js[^"'\''>\s]*' "$OUTPUT_DIR/reddit-home.html" >> "$OUTPUT_DIR/js-urls-raw.txt"
grep -oP '/_next/static/[^"'\''>\s]+' "$OUTPUT_DIR/reddit-home.html" | sed 's|^|https://www.reddit.com|' >> "$OUTPUT_DIR/js-urls-raw.txt"

# Deduplicate
sort -u "$OUTPUT_DIR/js-urls-raw.txt" > "$OUTPUT_DIR/js-urls.txt"
JS_COUNT=$(wc -l < "$OUTPUT_DIR/js-urls.txt")
log "  Found $JS_COUNT unique JS URLs"

if [ "$JS_COUNT" -eq 0 ]; then
    warn "No JS URLs found in HTML. Reddit may use RSC (React Server Components)."
    warn "Trying to extract script tags directly..."
    
    # Extract ALL script src attributes
    grep -oP '<script[^>]*src="([^"]*)"' "$OUTPUT_DIR/reddit-home.html" | sed 's/.*src="//;s/"//' >> "$OUTPUT_DIR/js-urls.txt"
    sort -u "$OUTPUT_DIR/js-urls.txt" -o "$OUTPUT_DIR/js-urls.txt"
    JS_COUNT=$(wc -l < "$OUTPUT_DIR/js-urls.txt")
    log "  Found $JS_COUNT JS URLs (after broader search)"
fi

# Show first few
head -10 "$OUTPUT_DIR/js-urls.txt" | while read -r url; do
    echo "    $url"
done

# === Step 4: Download all JS files ===
log ""
log "Step 4: Downloading JavaScript files..."

JS_DIR="$OUTPUT_DIR/reddit-js"
mkdir -p "$JS_DIR"

DOWNLOADED=0
FAILED=0
TOTAL=$(wc -l < "$OUTPUT_DIR/js-urls.txt")

while IFS= read -r url; do
    [ -z "$url" ] && continue
    
    # Make absolute URL if relative
    if [[ "$url" == /* ]]; then
        url="https://www.reddit.com$url"
    fi
    
    # Create safe filename
    FILENAME=$(echo "$url" | md5sum | cut -c1-12)
    FILENAME="${FILENAME}.js"
    
    HTTP_CODE=$(curl -s -o "$JS_DIR/$FILENAME" -w "%{http_code}" \
        -A "$UA" \
        -H "Accept: */*" \
        -H "Referer: https://www.reddit.com/" \
        --max-time 30 \
        "$url" 2>/dev/null)
    
    SIZE=$(wc -c < "$JS_DIR/$FILENAME" 2>/dev/null || echo 0)
    
    if [ "$HTTP_CODE" = "200" ] && [ "$SIZE" -gt 100 ]; then
        DOWNLOADED=$((DOWNLOADED + 1))
        echo -e "  ${GREEN}✓${NC} [$DOWNLOADED] ${url:0:80} (${SIZE} bytes)"
    else
        FAILED=$((FAILED + 1))
        rm -f "$JS_DIR/$FILENAME"
        echo -e "  ${RED}✗${NC} [$FAILED] ${url:0:80} (HTTP $HTTP_CODE)"
    fi
done < "$OUTPUT_DIR/js-urls.txt"

log "  Downloaded: $DOWNLOADED JS files ✓"
log "  Failed: $FAILED"

if [ "$DOWNLOADED" -eq 0 ]; then
    warn "No JS files downloaded. Trying capture_browser_scenario as fallback..."
    
    # === Fallback: Use REA capture_browser_scenario ===
    log ""
    log "Step 4b: Running REA capture_browser_scenario (fallback)..."
    
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
      "step_id": "wait_for_load",
      "action": "wait_for_timeout",
      "duration_ms": 15000
    }
  ],
  "capture": {
    "after_each_step": ["screenshot", "dom", "url"],
    "at_end": ["screenshot", "dom", "url"],
    "events": ["network", "console"],
    "network": {
      "request_body": false,
      "response_body": false,
      "header_values": true
    }
  }
}
SCENARIO

    log "  Running capture (a Chrome window will open)..."
    rea capture-browser-scenario "$OUTPUT_DIR/scenario.json" --format json > "$OUTPUT_DIR/capture-result.json" 2>"$OUTPUT_DIR/capture-errors.log"
    CAPTURE_EXIT=$?
    
    if [ $CAPTURE_EXIT -eq 0 ]; then
        log "  Capture successful! ✓"
        
        # Extract network events to find JS URLs
        python3 -c "
import json, os
d = json.load(open(os.path.expanduser('~/rea-output/capture-result.json')))
nr = d.get('normalized_result', {})
events = nr.get('events', {})
items = events.get('items', [])
print(f'Events: {len(items)}')
js_urls = set()
for item in items:
    if isinstance(item, dict):
        url = item.get('url', {})
        if isinstance(url, dict):
            u = url.get('url', '')
        else:
            u = str(url)
        if u and '.js' in u:
            js_urls.add(u)
if js_urls:
    with open(os.path.expanduser('~/rea-output/js-urls-from-capture.txt'), 'w') as f:
        for u in sorted(js_urls):
            f.write(u + '\n')
    print(f'JS URLs from capture: {len(js_urls)}')
else:
    print('No JS URLs in capture events')
" 2>/dev/null || warn "Could not parse capture result"
    else
        warn "Capture exit code: $CAPTURE_EXIT"
        cat "$OUTPUT_DIR/capture-errors.log" 2>/dev/null | head -5
    fi
fi

# === Step 5: Download service worker + robots.txt (always works) ===
log ""
log "Step 5: Downloading service worker + robots.txt..."
curl -sSL -A "$UA" "https://www.reddit.com/sw.js" -o "$OUTPUT_DIR/reddit-sw.js" --max-time 15
log "  Service worker: $(wc -c < "$OUTPUT_DIR/reddit-sw.js") bytes ✓"

curl -sSL -A "$UA" "https://www.reddit.com/robots.txt" -o "$OUTPUT_DIR/reddit-robots.txt" --max-time 10
log "  robots.txt: $(wc -c < "$OUTPUT_DIR/reddit-robots.txt") bytes ✓"

# === Step 6: Run REA analyze-javascript-application ===
log ""
log "Step 6: Running REA analyze-javascript-application..."

if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    JS_FILE_COUNT=$(ls "$JS_DIR" | wc -l)
    log "  Analyzing $JS_FILE_COUNT JS files..."
    
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUTPUT_DIR/analysis-evidence.json" 2>"$OUTPUT_DIR/analysis-errors.log"
    ANALYSIS_EXIT=$?
    
    if [ $ANALYSIS_EXIT -eq 0 ]; then
        log "  Analysis successful! ✓"
        ANALYSIS_SIZE=$(wc -c < "$OUTPUT_DIR/analysis-evidence.json")
        log "  Evidence: $ANALYSIS_SIZE bytes"
        
        # Show summary
        python3 -c "
import json, os
d = json.load(open(os.path.expanduser('~/rea-output/analysis-evidence.json')))
if 'error' in d:
    print(f'  Error: {d.get(\"message\", \"?\")}')
else:
    print(f'  Evidence ID: {d.get(\"evidence_id\", \"?\")[:50]}')
    nr = d.get('normalized_result', {})
    graph = nr.get('graph', {})
    sem = nr.get('semantic_graph', {})
    print(f'  Graph nodes: {len(graph.get(\"nodes\", []))}')
    print(f'  Graph edges: {len(graph.get(\"edges\", []))}')
    print(f'  Semantic nodes: {len(sem.get(\"nodes\", []))}')
    stats = nr.get('statistics', {})
    print(f'  Statistics: {stats}')
" 2>/dev/null || warn "Could not parse analysis"
    else
        warn "Analysis exit code: $ANALYSIS_EXIT"
        head -5 "$OUTPUT_DIR/analysis-errors.log" 2>/dev/null
    fi
else
    warn "No JS files to analyze"
fi

# === Step 7: Summary ===
log ""
log "========================================"
log "  CAPTURE COMPLETE"
log "========================================"
echo ""
echo "Output: $OUTPUT_DIR/"
echo ""
echo "Files:"
ls -lh "$OUTPUT_DIR/" 2>/dev/null | grep -v "^total" | grep -v "^d"
echo ""
if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    echo "JavaScript files: $(ls "$JS_DIR" | wc -l) files, $(du -sh "$JS_DIR" | awk '{print $1}')"
fi
echo ""
if [ -f "$OUTPUT_DIR/analysis-evidence.json" ]; then
    echo "REA analysis: $(ls -lh "$OUTPUT_DIR/analysis-evidence.json" | awk '{print $5}')"
fi
echo ""
echo "Next steps:"
echo "  1. View screenshot: xdg-open $OUTPUT_DIR/reddit-screenshot.png"
echo "  2. View HTML: less $OUTPUT_DIR/reddit-home.html"
echo "  3. View service worker: less $OUTPUT_DIR/reddit-sw.js"
echo "  4. View JS URLs: cat $OUTPUT_DIR/js-urls.txt"
echo "  5. If analysis succeeded, send me the analysis-evidence.json file"
echo "     and I'll use it to improve freeddit with real Reddit evidence"
echo ""
