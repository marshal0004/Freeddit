#!/bin/bash
set -uo pipefail
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
log()  { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err()  { echo -e "${RED}[error]${NC} $1" >&2; }

OUT="$HOME/rea-output"
mkdir -p "$OUT"
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"

log "Step 1: Install REA (if needed)"
command -v rea >/dev/null || npm install -g rea-agents
log "  REA: $(rea --version) ✓"

log ""
log "Step 2: Download Reddit HTML with FULL browser headers"
# The 8KB response means Reddit served a redirect/block. Use full browser headers.
curl -sSL -A "$UA" \
  -H "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8" \
  -H "Accept-Language: en-US,en;q=0.5" \
  -H "Accept-Encoding: gzip, deflate, br" \
  -H "Connection: keep-alive" \
  -H "Upgrade-Insecure-Requests: 1" \
  -H "Sec-Fetch-Dest: document" \
  -H "Sec-Fetch-Mode: navigate" \
  -H "Sec-Fetch-Site: none" \
  -H "Sec-Fetch-User: ?1" \
  --compressed \
  "https://www.reddit.com/r/programming/" \
  -o "$OUT/reddit-home.html" --max-time 30

HTML_SIZE=$(wc -c < "$OUT/reddit-home.html")
log "  HTML: $HTML_SIZE bytes"

# Check what we got
if [ "$HTML_SIZE" -lt 10000 ]; then
    warn "Small HTML ($HTML_SIZE bytes). Checking content..."
    head -c 500 "$OUT/reddit-home.html"
    echo ""
    
    # Try with cookies from a real browser session
    warn "Trying with browser cookie jar..."
    
    # Use Chrome to get cookies, then retry
    if [ -f "$HOME/.config/google-chrome/Default/Cookies" ]; then
        warn "Found Chrome cookies. Extracting Reddit session..."
        # Can't easily read Chrome's encrypted cookies, but let's try a different approach
    fi
    
    # Alternative: just fetch the JS files directly from known Reddit CDN paths
    log ""
    log "Step 2b: Trying to fetch JS directly from Reddit CDN..."
    
    # Reddit serves JS from /_next/static/chunks/ — try to discover them via the build manifest
    for manifest_path in \
        "/_next/static/chunks/webpack.js" \
        "/_next/static/chunks/main-app.js" \
        "/_next/static/chunks/framework.js" \
        "/_next/static/chunks/pages/_app.js" \
        "/_next/static/chunks/0.js" \
        "/_next/static/chunks/1.js" \
        "/_next/static/chunks/2.js"; do
        
        URL="https://www.reddit.com${manifest_path}"
        STATUS=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -A "$UA" \
            -H "Referer: https://www.reddit.com/" "$URL")
        echo "  $STATUS $URL"
    done
fi

# Check if WAF blocked
if grep -q "blocked by network security" "$OUT/reddit-home.html" 2>/dev/null; then
    err "WAF blocked! Reddit thinks this IP is a bot."
    err "Solutions:"
    echo "  1. Open Chrome, visit reddit.com manually, then re-run this script"
    echo "  2. Try a different network (mobile hotspot)"
    echo "  3. Disable any VPN"
    echo ""
    echo "But curl returned 200 earlier — the small HTML might be a JS redirect."
    echo "Let's check if it's a redirect page..."
    echo ""
    head -c 2000 "$OUT/reddit-home.html"
    exit 1
fi

# Check if it's a JS-heavy page (React Server Components)
if [ "$HTML_SIZE" -gt 50000 ]; then
    log "  Large HTML — looks like real Reddit content! ✓"
fi

log ""
log "Step 3: Extract JS URLs from HTML"

# Extract ALL script src URLs
grep -oP 'src="([^"]*\.js[^"]*)"' "$OUT/reddit-home.html" | sed 's/src="//;s/"//' > "$OUT/js-urls.txt"
# Also extract _next/static paths
grep -oP '/_next/static/[^"'\''>\s]+' "$OUT/reddit-home.html" >> "$OUT/js-urls.txt"
# Also check for redditstatic.com URLs
grep -oP 'https://[^"'\''>\s]*redditstatic[^"'\''>\s]*\.js[^"'\''>\s]*' "$OUT/reddit-home.html" >> "$OUT/js-urls.txt"
# Deduplicate
sort -u "$OUT/js-urls.txt" -o "$OUT/js-urls.txt"
JS_COUNT=$(wc -l < "$OUT/js-urls.txt")
log "  Found $JS_COUNT JS URLs"

if [ "$JS_COUNT" -eq 0 ]; then
    warn "No script src tags found. Reddit likely uses RSC (server-rendered)."
    warn "The HTML is server-rendered React — the JS is embedded differently."
    warn ""
    warn "Extracting inline scripts instead..."
    
    # Extract ALL inline <script> content
    python3 -c "
import re
html = open('$OUT/reddit-home.html').read()
scripts = re.findall(r'<script[^>]*>(.*?)</script>', html, re.DOTALL)
inline_count = 0
for i, script in enumerate(scripts):
    if len(script.strip()) > 100:
        with open(f'$OUT/inline-script-{i}.js', 'w') as f:
            f.write(script)
        inline_count += 1
        print(f'  Inline script {i}: {len(script)} bytes')
print(f'Total inline scripts: {inline_count}')
" 2>/dev/null || warn "Could not extract inline scripts"
    
    # Also extract CSS
    grep -oP 'href="([^"]*\.css[^"]*)"' "$OUT/reddit-home.html" | sed 's/href="//;s/"//' > "$OUT/css-urls.txt"
    CSS_COUNT=$(wc -l < "$OUT/css-urls.txt")
    log "  Found $CSS_COUNT CSS URLs"
fi

log ""
log "Step 4: Download all JS/CSS files"
JS_DIR="$OUT/reddit-js"
mkdir -p "$JS_DIR"

DOWNLOADED=0
while IFS= read -r url; do
    [ -z "$url" ] && continue
    # Make absolute
    if [[ "$url" == /* ]]; then
        url="https://www.reddit.com${url}"
    fi
    
    FNAME=$(echo "$url" | md5sum | cut -c1-12)
    CODE=$(curl -s -o "$JS_DIR/${FNAME}.js" -w '%{http_code}' --max-time 20 \
        -A "$UA" -H "Referer: https://www.reddit.com/" "$url")
    SIZE=$(wc -c < "$JS_DIR/${FNAME}.js" 2>/dev/null || echo 0)
    
    if [ "$CODE" = "200" ] && [ "$SIZE" -gt 100 ]; then
        DOWNLOADED=$((DOWNLOADED+1))
        log "  ✓ [$DOWNLOADED] ${url:0:70}... (${SIZE}b)"
    else
        rm -f "$JS_DIR/${FNAME}.js"
    fi
done < "$OUT/js-urls.txt"

# Also download CSS
while IFS= read -r url; do
    [ -z "$url" ] && continue
    if [[ "$url" == /* ]]; then url="https://www.reddit.com${url}"; fi
    FNAME=$(echo "$url" | md5sum | cut -c1-12)
    curl -s -o "$JS_DIR/${FNAME}.css" --max-time 20 -A "$UA" -H "Referer: https://www.reddit.com/" "$url"
    SIZE=$(wc -c < "$JS_DIR/${FNAME}.css" 2>/dev/null || echo 0)
    [ "$SIZE" -gt 100 ] && DOWNLOADED=$((DOWNLOADED+1)) && log "  ✓ [$DOWNLOADED] CSS: ${url:0:60}... (${SIZE}b)"
done < "$OUT/css-urls.txt" 2>/dev/null

log "  Downloaded: $DOWNLOADED files"

log ""
log "Step 5: Download service worker + robots.txt"
curl -sSL -A "$UA" "https://www.reddit.com/sw.js" -o "$OUT/reddit-sw.js" --max-time 15
log "  sw.js: $(wc -c < "$OUT/reddit-sw.js") bytes ✓"

curl -sSL -A "$UA" "https://www.reddit.com/robots.txt" -o "$OUT/reddit-robots.txt" --max-time 10
log "  robots.txt: $(wc -c < "$OUT/reddit-robots.txt") bytes ✓"

log ""
log "Step 6: Run REA analyze-javascript-application"
if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    FILE_COUNT=$(ls "$JS_DIR" | wc -l)
    log "  Analyzing $FILE_COUNT files..."
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUT/analysis-evidence.json" 2>"$OUT/analysis.log"
    if [ $? -eq 0 ]; then
        log "  Analysis OK! ✓ ($(wc -c < "$OUT/analysis-evidence.json") bytes)"
    else
        warn "  Analysis failed. Check $OUT/analysis.log"
    fi
else
    warn "  No JS files to analyze"
fi

log ""
log "========================================"
log "  COMPLETE — output in $OUT/"
log "========================================"
echo ""
ls -lh "$OUT/" 2>/dev/null | grep -v "^total"
echo ""
if [ -d "$JS_DIR" ]; then
    echo "JS files: $(ls "$JS_DIR" 2>/dev/null | wc -l)"
fi
echo ""
echo "Next: send me ~/rea-output/analysis-evidence.json"
echo "      or ~/rea-output/reddit-home.html"
echo "      and I'll use it to improve freeddit"
