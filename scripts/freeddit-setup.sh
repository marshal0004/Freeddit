#!/bin/bash
# freeddit-setup.sh — Arch Linux installer for freeddit
# Installs all dependencies, builds the Rust backend + Next.js frontend, and starts the app.
#
# Usage:
#   sudo ./freeddit-setup.sh           # system install
#   ./freeddit-setup.sh --user         # user-local install (no sudo)
#
# After install, freeddit is started with:
#   freeddit                            # backend + frontend
#   freeddit-electron                   # Electron desktop wrapper

set -euo pipefail

# === Colors ===
RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
BLUE=$'\033[0;34m'
NC=$'\033[0m'

log() { echo -e "${GREEN}[freeddit]${NC} $1"; }
warn() { echo -e "${YELLOW}[warn]${NC} $1"; }
err() { echo -e "${RED}[error]${NC} $1" >&2; }

# === Parse args ===
USER_MODE=0
if [[ "${1:-}" == "--user" ]]; then
  USER_MODE=1
fi

if [[ $EUID -eq 0 ]] && [[ $USER_MODE -eq 0 ]]; then
  log "Running in system mode (sudo)"
elif [[ $EUID -ne 0 ]] && [[ $USER_MODE -eq 0 ]]; then
  warn "Not running as root. Use --user for user-local install, or re-run with sudo."
  exit 1
fi

# === Detect package manager ===
if command -v pacman >/dev/null 2>&1; then
  PM="pacman"
elif command -v apt >/dev/null 2>&1; then
  PM="apt"
elif command -v dnf >/dev/null 2>&1; then
  PM="dnf"
else
  err "Unsupported distro: no pacman/apt/dnf found."
  exit 1
fi
log "Detected package manager: $PM"

# === Install system deps ===
install_deps() {
  case "$PM" in
    pacman)
      local pkgs=(rust nodejs npm bun sqlite chromium caddy git base-devel openssl pkgconf)
      # Check if bun is in AUR
      if ! pacman -Si bun >/dev/null 2>&1; then
        warn "bun not in official repos — will install via npm"
        pkgs=(rust nodejs npm sqlite chromium caddy git base-devel openssl pkgconf)
      fi
      if [[ $USER_MODE -eq 0 ]]; then
        sudo pacman -Sy --noconfirm --needed "${pkgs[@]}"
      else
        warn "User mode: please install system packages manually:"
        echo "  sudo pacman -S ${pkgs[*]}"
        return 1
      fi
      ;;
    apt)
      local pkgs=(build-essential curl libssl-dev pkg-config nodejs npm git sqlite3 chromium-browser caddy)
      if [[ $USER_MODE -eq 0 ]]; then
        sudo apt update
        sudo apt install -y "${pkgs[@]}"
        # Install Rust via rustup
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
        source "$HOME/.cargo/env"
      fi
      ;;
    dnf)
      local pkgs=(gcc cargo nodejs npm git sqlite chromium caddy openssl-devel pkgconf-pkg-config)
      if [[ $USER_MODE -eq 0 ]]; then
        sudo dnf install -y "${pkgs[@]}"
      fi
      ;;
  esac
}

# === Clone or update repo ===
clone_repo() {
  local dest="$1"
  if [[ -d "$dest/.git" ]]; then
    log "Updating existing clone at $dest"
    cd "$dest" && git pull
  else
    log "Cloning freeddit to $dest"
    git clone https://github.com/marshal0004/Freeddit.git "$dest"
  fi
}

# === Build backend ===
build_backend() {
  local dest="$1"
  log "Building Rust backend (this takes 1-3 minutes)..."
  cd "$dest/backend-rust"
  cargo build --release
  log "Backend binary: $(ls -lh target/release/freeddit-backend | awk '{print $5}')"
}

# === Build frontend ===
build_frontend() {
  local dest="$1"
  log "Installing npm deps..."
  cd "$dest"
  if command -v bun >/dev/null 2>&1; then
    bun install
  else
    npm install
  fi
  log "Building Next.js production bundle..."
  if command -v bun >/dev/null 2>&1; then
    bun run build
  else
    npm run build
  fi
}

# === Create launcher scripts ===
create_launchers() {
  local dest="$1"
  local bin_dir="$HOME/.local/bin"
  mkdir -p "$bin_dir"

  # freeddit launcher (starts backend + frontend + opens browser)
  cat > "$bin_dir/freeddit" <<EOF
#!/bin/bash
# freeddit launcher — starts backend, dev server, opens browser
set -e
DEST="$dest"
cd "\$DEST"

# Kill any existing freeddit processes
pkill -f "freeddit-backend" 2>/dev/null || true
pkill -f "next start" 2>/dev/null || true
sleep 1

# Start backend
DATABASE_URL="sqlite://\$DEST/freeddit.db?mode=rwc" PORT=3001 \\
  "\$DEST/backend-rust/target/release/freeddit-backend" > "\$DEST/backend.log" 2>&1 &
BACKEND_PID=\$!
echo "Backend PID: \$BACKEND_PID (log: \$DEST/backend.log)"

# Wait for backend
for i in {1..30}; do
  curl -s --max-time 1 http://localhost:3001/api/health 2>/dev/null | grep -q ok && break
  sleep 1
done

# Start frontend
cd "\$DEST"
if command -v bun >/dev/null 2>&1; then
  bun run start > "\$DEST/frontend.log" 2>&1 &
else
  npm run start > "\$DEST/frontend.log" 2>&1 &
fi
FRONTEND_PID=\$!
echo "Frontend PID: \$FRONTEND_PID (log: \$DEST/frontend.log)"

# Open browser
sleep 3
xdg-open "http://localhost:3000/" 2>/dev/null || true

echo ""
echo "freeddit is running at http://localhost:3000/"
echo "Backend API at http://localhost:3001/api/"
echo "Stop with: kill \$BACKEND_PID \$FRONTEND_PID"
echo ""
echo "Press Ctrl+C to stop..."
trap "kill \$BACKEND_PID \$FRONTEND_PID 2>/dev/null; exit" INT TERM
wait
EOF
  chmod +x "$bin_dir/freeddit"
  log "Created launcher: $bin_dir/freeddit"

  # freeddit-dev launcher (dev mode with hot reload)
  cat > "$bin_dir/freeddit-dev" <<EOF
#!/bin/bash
set -e
DEST="$dest"
cd "\$DEST"
pkill -f "freeddit-backend" 2>/dev/null || true
pkill -f "next dev" 2>/dev/null || true
sleep 1
DATABASE_URL="sqlite://\$DEST/freeddit.db?mode=rwc" PORT=3001 \\
  "\$DEST/backend-rust/target/release/freeddit-backend" > "\$DEST/backend.log" 2>&1 &
BACKEND_PID=\$!
cd "\$DEST"
if command -v bun >/dev/null 2>&1; then
  bun run dev > "\$DEST/dev.log" 2>&1 &
else
  npm run dev > "\$DEST/dev.log" 2>&1 &
fi
DEV_PID=\$!
sleep 5
xdg-open "http://localhost:3000/" 2>/dev/null || true
echo "freeddit dev mode: http://localhost:3000/ (Ctrl+C to stop)"
trap "kill \$BACKEND_PID \$DEV_PID 2>/dev/null; exit" INT TERM
wait
EOF
  chmod +x "$bin_dir/freeddit-dev"
  log "Created launcher: $bin_dir/freeddit-dev"

  # freeddit-electron launcher
  cat > "$bin_dir/freeddit-electron" <<EOF
#!/bin/bash
set -e
DEST="$dest"
cd "\$DEST/electron"
if [[ ! -d node_modules ]]; then
  npm install
fi
# Start backend + frontend first
\$HOME/.local/bin/freeddit &
APP_PID=\$!
sleep 5
# Launch electron pointing to local Next.js
npx electron . --dev 2>&1
trap "kill \$APP_PID 2>/dev/null; exit" INT TERM
EOF
  chmod +x "$bin_dir/freeddit-electron"
  log "Created launcher: $bin_dir/freeddit-electron"
}

# === Create desktop entry ===
create_desktop_entry() {
  local dest="$1"
  local apps_dir="$HOME/.local/share/applications"
  mkdir -p "$apps_dir"
  cat > "$apps_dir/freeddit.desktop" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=freeddit
Comment=An open-source Reddit clone (Rust + Next.js + Electron)
Exec=$HOME/.local/bin/freeddit
Icon=$dest/public/freeddit-logo.svg
Terminal=true
Categories=Network;News;
Keywords=reddit;social;news;community;
EOF
  log "Created desktop entry: $apps_dir/freeddit.desktop"
}

# === Main ===
main() {
  log "freeddit installer for Arch Linux (and other distros)"
  echo ""

  # Install deps
  if [[ $USER_MODE -eq 0 ]]; then
    install_deps
  else
    warn "User mode: skipping system deps. Install manually: rust nodejs npm bun sqlite caddy"
  fi
  echo ""

  # Clone
  DEST="${FREEDDIT_HOME:-$HOME/freeddit}"
  clone_repo "$DEST"
  echo ""

  # Build
  build_backend "$DEST"
  echo ""
  build_frontend "$DEST"
  echo ""

  # Launchers + desktop entry
  create_launchers "$DEST"
  create_desktop_entry "$DEST"

  echo ""
  log "✅ freeddit installed successfully!"
  echo ""
  echo -e "${BLUE}Usage:${NC}"
  echo "  freeddit              # Start freeddit (production mode)"
  echo "  freeddit-dev          # Start in dev mode (hot reload)"
  echo "  freeddit-electron     # Launch as Electron desktop app"
  echo ""
  echo -e "${BLUE}Location:${NC} $DEST"
  echo -e "${BLUE}Default URL:${NC} http://localhost:3000/"
  echo ""
  echo -e "${BLUE}Default seed user:${NC} alice / password123"
  echo ""
  echo -e "${BLUE}Docs:${NC} $DEST/docs/ARCHITECTURE.md"
  echo ""
  echo -e "${BLUE}First-time setup:${NC}"
  echo "  1. Run 'freeddit-dev' (or 'freeddit') to start the app"
  echo "  2. Open http://localhost:3000/ in your browser"
  echo "  3. Click 'Log In' and use alice / password123"
  echo "  4. Or click 'Sign Up' to create your own account"
  echo ""
}

main "$@"
