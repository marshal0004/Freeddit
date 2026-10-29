#!/bin/bash
#
# rea-reddit-capture.sh — Download Reddit's JavaScript with a REAL browser
#
# WHY THIS APPROACH: Reddit uses nonce-based script protection.
# curl gets 403 on JS files. Only a real browser can load them.
# This script uses Playwright (real Chrome) to intercept and save every JS file.
#
# Usage:
#   chmod +x scripts/rea-reddit-capture.sh
#   ./scripts/rea-reddit-capture.sh
#

set -uo pipefail
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
log()  { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err()  { echo -e "${RED}[error]${NC} $1" >&2; }

OUT="$HOME/rea-output"
mkdir -p "$OUT"

# Step 1: Install Playwright
log "Step 1: Installing Playwright..."
npm install -g playwright 2>/dev/null || npx playwright install chromium 2>/dev/null
log "  Playwright ready ✓"

# Step 2: Run the Node.js capture script
log ""
log "Step 2: Launching Chrome to capture Reddit's JavaScript..."
log "  (A Chrome window will open and navigate to reddit.com)"
log ""

node "$(dirname "$0")/rea-reddit-capture.js"

EXIT=$?
if [ $EXIT -ne 0 ]; then
    err "Capture script failed. Trying with npx..."
    npx playwright install chromium 2>/dev/null
    node "$(dirname "$0")/rea-reddit-capture.js"
    EXIT=$?
fi

# Step 3: Also download service worker + robots.txt
log ""
log "Step 3: Downloading service worker + robots.txt..."
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
curl -sSL -A "$UA" "https://www.reddit.com/sw.js" -o "$OUT/reddit-sw.js" --max-time 15 2>/dev/null
log "  sw.js: $(wc -c < "$OUT/reddit-sw.js" 2>/dev/null || echo 0) bytes ✓"

curl -sSL -A "$UA" "https://www.reddit.com/robots.txt" -o "$OUT/reddit-robots.txt" --max-time 10 2>/dev/null
log "  robots.txt: $(wc -c < "$OUT/reddit-robots.txt" 2>/dev/null || echo 0) bytes ✓"

# Step 4: Run REA analysis if JS files were downloaded
JS_DIR="$OUT/reddit-js"
if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    FILE_COUNT=$(ls "$JS_DIR" | wc -l)
    log ""
    log "Step 4: Running REA analyze-javascript-application on $FILE_COUNT files..."
    
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUT/analysis-evidence.json" 2>"$OUT/analysis-errors.log"
    
    if [ $? -eq 0 ]; then
        log "  Analysis successful! ✓"
        log "  Evidence: $(wc -c < "$OUT/analysis-evidence.json") bytes"
        log ""
        log "DONE! Send me these files:"
        echo "  ~/rea-output/analysis-evidence.json"
        echo "  ~/rea-output/reddit-screenshot.png"
        echo "  ~/rea-output/js-urls.txt"
    else
        warn "  Analysis failed. Check $OUT/analysis-errors.log"
    fi
else
    warn "No JS files captured. Try opening Chrome manually first:"
    echo "  1. Open Chrome"
    echo "  2. Go to https://www.reddit.com/r/programming/"
    echo "  3. Wait for it to fully load"
    echo "  4. Close Chrome"
    echo "  5. Re-run this script"
fi

echo ""
