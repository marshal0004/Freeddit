#!/bin/bash
#
# rea-reddit-apk.sh — Download + Analyze Reddit's Android APK with REA
#
# WHY APK: Reddit's website has WAF + CAPTCHA that blocks automated browsers.
# The Android APK is a static file — no anti-bot detection possible.
# It contains ALL of Reddit's business logic: API calls, data models,
# ranking algorithms, UI components, etc.
#
# WHAT THIS DOES:
# 1. Downloads Reddit APK (com.reddit.frontpage)
# 2. Installs JADX (Java decompiler — free, open-source)
# 3. Decompiles the APK to Java source code
# 4. Runs REA inspect-android-package on the APK
# 5. Extracts all API endpoints, data models, feature logic
# 6. Runs REA analyze-javascript-application on any embedded JS
#
# Usage:
#   chmod +x scripts/rea-reddit-apk.sh
#   ./scripts/rea-reddit-apk.sh
#

set -uo pipefail
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
log()  { echo -e "${GREEN}[REA]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err()  { echo -e "${RED}[error]${NC} $1" >&2; }

OUT="$HOME/rea-output"
APK_DIR="$OUT/reddit-apk"
SRC_DIR="$OUT/reddit-source"
mkdir -p "$OUT" "$APK_DIR" "$SRC_DIR"

# === Step 1: Check prerequisites ===
log "Step 1: Checking prerequisites..."

# Check Java (needed for JADX)
if ! command -v java &>/dev/null; then
    err "Java not found. Install: sudo pacman -S jdk-openjdk"
    exit 1
fi
log "  Java: $(java -version 2>&1 | head -1) ✓"

# Check REA
if ! command -v rea &>/dev/null; then
    log "  Installing REA..."
    npm install -g rea-agents
fi
log "  REA: $(rea --version) ✓"

# === Step 2: Download Reddit APK ===
log ""
log "Step 2: Download Reddit APK"

APK_FILE="$APK_DIR/reddit.apk"

if [ -f "$APK_FILE" ]; then
    log "  APK already exists: $(ls -lh "$APK_FILE" | awk '{print $5}')"
else
    log "  Please download the Reddit APK manually:"
    echo ""
    echo -e "  ${BLUE}Option A: Download from APKMirror${NC}"
    echo "    1. Open: https://www.apkmirror.com/apk/redditinc/reddit/"
    echo "    2. Click the latest version"
    echo "    3. Download the APK (choose the 'APK' variant, not 'APK Bundle')"
    echo "    4. Move it to: $APK_FILE"
    echo ""
    echo -e "  ${BLUE}Option B: Download from APKPure${NC}"
    echo "    1. Open: https://apkpure.com/reddit/com.reddit.frontpage"
    echo "    2. Click 'Download APK'"
    echo "    3. Move it to: $APK_FILE"
    echo ""
    echo -e "  ${BLUE}Option C: Use adb to pull from your phone${NC}"
    echo "    adb shell pm path com.reddit.frontpage"
    echo "    adb pull /data/app/~~/base.apk $APK_FILE"
    echo ""
    echo -e "  ${BLUE}Option D: Download via command line${NC}"
    echo "    # If you have apkeep installed:"
    echo "    apkeep -a com.reddit.frontpage -d apkpure $APK_DIR"
    echo ""
    read -p "  Press ENTER after you've placed the APK at $APK_FILE..."
fi

# Verify APK exists
if [ ! -f "$APK_FILE" ]; then
    # Check for any .apk in the directory
    FOUND_APK=$(find "$APK_DIR" -name "*.apk" -type f 2>/dev/null | head -1)
    if [ -n "$FOUND_APK" ]; then
        APK_FILE="$FOUND_APK"
        log "  Found APK: $APK_FILE ($(ls -lh "$APK_FILE" | awk '{print $5}'))"
    else
        err "No APK file found in $APK_DIR"
        exit 1
    fi
fi

# Verify it's a valid APK
if ! unzip -l "$APK_FILE" 2>/dev/null | grep -q "classes.dex"; then
    err "File doesn't look like an APK (no classes.dex found)"
    err "Make sure you downloaded the APK variant, not an XAPK or APK Bundle"
    exit 1
fi
log "  APK verified ✓ ($(ls -lh "$APK_FILE" | awk '{print $5}'))"

# === Step 3: Install JADX ===
log ""
log "Step 3: Install JADX (Java decompiler)..."

JADX_DIR="$HOME/jadx"
if [ ! -d "$JADX_DIR" ] || [ ! -f "$JADX_DIR/bin/jadx" ]; then
    log "  Downloading JADX..."
    
    # Get latest JADX release URL
    JADX_URL=$(curl -sL "https://api.github.com/repos/skylot/jadx/releases/latest" 2>/dev/null | \
        python3 -c "
import json, sys
d = json.load(sys.stdin)
for asset in d.get('assets', []):
    if 'no-jre' not in asset['name'] and 'linux' in asset['name'] and asset['name'].endswith('.zip'):
        print(asset['browser_download_url'])
        break
" 2>/dev/null)
    
    if [ -z "$JADX_URL" ]; then
        # Fallback: try a known version
        JADX_URL="https://github.com/skylot/jadx/releases/download/v1.5.1/jadx-1.5.1.zip"
    fi
    
    log "  Downloading from: ${JADX_URL:0:80}..."
    curl -sSL "$JADX_URL" -o /tmp/jadx.zip --max-time 60
    mkdir -p "$JADX_DIR"
    unzip -q -o /tmp/jadx.zip -d "$JADX_DIR"
    rm -f /tmp/jadx.zip
fi

JADX_BIN="$JADX_DIR/bin/jadx"
if [ ! -f "$JADX_BIN" ]; then
    # Try alternative location
    JADX_BIN=$(find "$JADX_DIR" -name "jadx" -type f 2>/dev/null | head -1)
fi

if [ -z "$JADX_BIN" ] || [ ! -f "$JADX_BIN" ]; then
    err "JADX not found. Install manually: sudo pacman -S jadx"
    exit 1
fi
log "  JADX: $JADX_BIN ✓"

# Set JADX for REA
export REA_JADX_MCP_JAR=$(find "$JADX_DIR" -name "jadx-cli*.jar" -o -name "jadx-*.jar" 2>/dev/null | head -1)
log "  JADX JAR: $REA_JADX_MCP_JAR"

# === Step 4: Decompile APK with JADX ===
log ""
log "Step 4: Decompiling APK with JADX..."
log "  (This takes 1-5 minutes depending on APK size)"

rm -rf "$SRC_DIR"
mkdir -p "$SRC_DIR"

"$JADX_BIN" \
    --deobf \
    --show-bad-code \
    --no-imports \
    --output-dir "$SRC_DIR" \
    "$APK_FILE" 2>&1 | tail -10

# Count decompiled files
JAVA_FILES=$(find "$SRC_DIR" -name "*.java" -type f 2>/dev/null | wc -l)
log "  Decompiled: $JAVA_FILES Java files ✓"

# Show directory structure
log ""
log "  Source structure:"
find "$SRC_DIR" -type d | head -20 | while read -r dir; do
    COUNT=$(find "$dir" -maxdepth 1 -name "*.java" -type f 2>/dev/null | wc -l)
    [ "$COUNT" -gt 0 ] && echo "    $dir ($COUNT .java files)"
done | head -15

# === Step 5: Extract key information ===
log ""
log "Step 5: Extracting Reddit's API + features from decompiled code..."

# Extract API endpoints
log "  Searching for API endpoints..."
grep -rn "api\.\|/api/\|graphql\|/svc/" "$SRC_DIR" --include="*.java" 2>/dev/null | \
    grep -oP '"[^"]*api[^"]*"' | sort -u > "$OUT/api-endpoints.txt" 2>/dev/null
API_COUNT=$(wc -l < "$OUT/api-endpoints.txt" 2>/dev/null || echo 0)
log "    Found $API_COUNT API endpoints"

# Extract data models (classes with "Model", "DTO", "Entity" in name)
log "  Searching for data models..."
find "$SRC_DIR" -name "*Model*.java" -o -name "*DTO*.java" -o -name "*Entity*.java" -o -name "*Response*.java" 2>/dev/null | \
    head -30 > "$OUT/data-models.txt"
MODEL_COUNT=$(wc -l < "$OUT/data-models.txt" 2>/dev/null || echo 0)
log "    Found $MODEL_COUNT data model classes"

# Extract feature-related classes
log "  Searching for feature classes..."
grep -rln "vote\|comment\|subreddit\|post\|ranking\|sort\|notification\|message\|search" \
    "$SRC_DIR" --include="*.java" -i 2>/dev/null | \
    head -50 > "$OUT/feature-classes.txt"
FEATURE_COUNT=$(wc -l < "$OUT/feature-classes.txt" 2>/dev/null || echo 0)
log "    Found $FEATURE_COUNT feature-related classes"

# Extract native libraries
log ""
log "  Checking for native libraries (.so files)..."
SO_DIR="$OUT/native-libs"
mkdir -p "$SO_DIR"
unzip -o "$APK_FILE" "lib/*/*.so" -d "$OUT/apk-extracted" 2>/dev/null
SO_COUNT=$(find "$OUT/apk-extracted" -name "*.so" -type f 2>/dev/null | wc -l)
if [ "$SO_COUNT" -gt 0 ]; then
    log "    Found $SO_COUNT native libraries:"
    find "$OUT/apk-extracted" -name "*.so" -type f 2>/dev/null | while read -r so; do
        echo "      $(basename "$so") ($(ls -lh "$so" | awk '{print $5}'))"
    done
else
    log "    No native libraries in this APK"
fi

# Extract embedded JavaScript (some apps have WebView JS)
log ""
log "  Checking for embedded JavaScript..."
JS_DIR="$OUT/embedded-js"
mkdir -p "$JS_DIR"
unzip -o "$APK_FILE" "assets/*.js" "assets/**/*.js" -d "$OUT/apk-extracted" 2>/dev/null
EMBEDDED_JS=$(find "$OUT/apk-extracted" -name "*.js" -type f 2>/dev/null | wc -l)
if [ "$EMBEDDED_JS" -gt 0 ]; then
    log "    Found $EMBEDDED_JS JavaScript files — copying..."
    find "$OUT/apk-extracted" -name "*.js" -type f -exec cp {} "$JS_DIR/" \; 2>/dev/null
else
    log "    No embedded JavaScript"
fi

# === Step 6: Run REA analysis ===
log ""
log "Step 6: Running REA analysis..."

# Try REA's inspect-android-package
log "  Running rea inspect-android-package..."
rea inspect-android-package "$APK_FILE" --format json > "$OUT/apk-analysis.json" 2>"$OUT/apk-analysis-errors.log"
if [ $? -eq 0 ]; then
    log "  APK analysis OK! ✓"
    log "  Evidence: $(wc -c < "$OUT/apk-analysis.json") bytes"
else
    warn "  REA inspect-android-package failed (JADX MCP may not be configured)"
    warn "  Check: $OUT/apk-analysis-errors.log"
    warn ""
    warn "  You can still analyze the decompiled source with REA:"
    echo "    rea analyze-javascript-application $SRC_DIR --format json"
fi

# If we found embedded JS, analyze it too
if [ "$EMBEDDED_JS" -gt 0 ]; then
    log ""
    log "  Running REA on embedded JavaScript..."
    rea analyze-javascript-application "$JS_DIR" --format json > "$OUT/js-analysis.json" 2>/dev/null
    [ $? -eq 0 ] && log "  JS analysis OK! ✓"
fi

# === Step 7: Run nm/strings on native libraries (Ghidra-style) ===
if [ "$SO_COUNT" -gt 0 ]; then
    log ""
    log "Step 7: Analyzing native libraries with nm/strings..."
    
    SO_ANALYSIS="$OUT/native-analysis"
    mkdir -p "$SO_ANALYSIS"
    
    find "$OUT/apk-extracted" -name "*.so" -type f | while read -r so; do
        SONAME=$(basename "$so")
        log "  Analyzing: $SONAME"
        
        # Extract function names
        nm -D "$so" 2>/dev/null | grep " T " | awk '{print $3}' > "$SO_ANALYSIS/${SONAME}-functions.txt"
        FUNC_COUNT=$(wc -l < "$SO_ANALYSIS/${SONAME}-functions.txt" 2>/dev/null || echo 0)
        log "    Functions: $FUNC_COUNT"
        
        # Extract strings
        strings "$so" 2>/dev/null > "$SO_ANALYSIS/${SONAME}-strings.txt"
        STR_COUNT=$(wc -l < "$SO_ANALYSIS/${SONAME}-strings.txt" 2>/dev/null || echo 0)
        log "    Strings: $STR_COUNT"
        
        # Search for Reddit-specific strings
        grep -iE "reddit|api\.|graphql|vote|comment|subreddit|ranking|score|karma" \
            "$SO_ANALYSIS/${SONAME}-strings.txt" 2>/dev/null > "$SO_ANALYSIS/${SONAME}-relevant.txt"
        REL_COUNT=$(wc -l < "$SO_ANALYSIS/${SONAME}-relevant.txt" 2>/dev/null || echo 0)
        log "    Reddit-relevant: $REL_COUNT"
    done
fi

# === Summary ===
log ""
log "========================================"
log "  ANALYSIS COMPLETE"
log "========================================"
echo ""
echo "Output: $OUT/"
echo ""
echo "Files:"
ls -lh "$OUT/"*.txt "$OUT/"*.json "$OUT/"*.log 2>/dev/null | grep -v "^total"
echo ""
echo "Decompiled source: $SRC_DIR/"
echo "  Java files: $JAVA_FILES"
echo "  API endpoints: $API_COUNT"
echo "  Data models: $MODEL_COUNT"
echo "  Feature classes: $FEATURE_COUNT"
echo ""
if [ "$SO_COUNT" -gt 0 ]; then
    echo "Native libraries: $SO_COUNT .so files"
    echo "  Analysis: $OUT/native-analysis/"
fi
echo ""
echo -e "${BLUE}NEXT STEPS:${NC}"
echo ""
echo "1. Review API endpoints:"
echo "   less $OUT/api-endpoints.txt"
echo ""
echo "2. Review decompiled source (search for features):"
echo "   grep -rn 'vote\|upvote\|downvote' $SRC_DIR --include='*.java'"
echo "   grep -rn 'comment.*sort\|best\|top\|new\|controversial' $SRC_DIR --include='*.java'"
echo "   grep -rn 'ranking\|hot_score\|best_score\|algorithm' $SRC_DIR --include='*.java'"
echo ""
echo "3. Send me the results:"
echo "   cat $OUT/api-endpoints.txt"
echo "   cat $OUT/feature-classes.txt"
echo "   # Or zip everything:"
echo "   zip -r reddit-analysis.zip $OUT/"
echo ""
echo "4. I'll use this evidence to improve freeddit!"
echo ""
