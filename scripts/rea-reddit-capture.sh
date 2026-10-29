#!/bin/bash
set -uo pipefail
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
log()  { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err()  { echo -e "${RED}[error]${NC} $1" >&2; }

OUT="$HOME/rea-output"
mkdir -p "$OUT"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Step 1: Install playwright locally in scripts dir
log "Step 1: Installing Playwright..."
cd "$SCRIPT_DIR"
if [ ! -d node_modules/playwright ]; then
    npm init -y >/dev/null 2>&1
    npm install playwright 2>&1 | tail -3
fi
log "  Playwright ready ✓"

# Step 2: Run the capture script
log ""
log "Step 2: Launching Chrome to capture Reddit's JavaScript..."
node "$SCRIPT_DIR/rea-reddit-capture.js"
EXIT=$?

# Step 3: Also download service worker
log ""
log "Step 3: Downloading service worker..."
curl -sSL "https://www.reddit.com/sw.js" -o "$OUT/reddit-sw.js" --max-time 15 2>/dev/null
log "  sw.js: $(wc -c < "$OUT/reddit-sw.js") bytes ✓"

# Step 4: Run REA analysis
JS_DIR="$OUT/reddit-js"
if [ -d "$JS_DIR" ] && [ "$(ls -A "$JS_DIR" 2>/dev/null)" ]; then
    FILE_COUNT=$(ls "$JS_DIR" | wc -l)
    log ""
    log "Step 4: Running REA analysis on $FILE_COUNT JS files..."
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUT/analysis-evidence.json" 2>"$OUT/analysis-errors.log"
    if [ $? -eq 0 ]; then
        log "  Analysis OK! ✓ ($(wc -c < "$OUT/analysis-evidence.json") bytes)"
        echo ""
        echo "DONE! Send me: ~/rea-output/analysis-evidence.json"
    else
        warn "  Analysis failed. Check $OUT/analysis-errors.log"
    fi
else
    warn "No JS files captured."
fi
echo ""
