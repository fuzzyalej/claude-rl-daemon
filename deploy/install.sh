#!/usr/bin/env bash
set -euo pipefail

BINARY="target/release/claude-rl-daemon"
INSTALL_DIR="$HOME/.local/bin"
INSTALL_PATH="$INSTALL_DIR/claude-rl-daemon"
PLIST_DST="$HOME/Library/LaunchAgents/com.claudedaemon.plist"

echo "Building release binary..."
cargo build --release

echo "Installing binary to $INSTALL_PATH"
mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

CDAEMON_SRC="target/release/cdaemon"
CDAEMON_PATH="$INSTALL_DIR/cdaemon"

echo "Installing cdaemon to $CDAEMON_PATH"
cp "$CDAEMON_SRC" "$CDAEMON_PATH"
chmod +x "$CDAEMON_PATH"

echo "Creating log directory..."
mkdir -p "$HOME/.claude-daemon"

echo "Installing launchd plist..."
mkdir -p "$HOME/Library/LaunchAgents"

# Generate plist with the fully-expanded binary path (launchd doesn't expand ~)
sed -e "s|__INSTALL_PATH__|$INSTALL_PATH|g" \
    -e "s|__HOME__|$HOME|g" \
    deploy/com.claudedaemon.plist > "$PLIST_DST"

# Unload if already loaded (idempotent)
launchctl unload "$PLIST_DST" 2>/dev/null || true
launchctl load "$PLIST_DST"

echo "Done. Daemon is running."
echo ""
echo "Useful commands:"
echo "  Check status:  cdaemon status"
echo "  View logs:     cdaemon logs --follow"
echo "  List sessions: cdaemon sessions"
echo "  Doctor check:  cdaemon doctor"
echo "  Stop daemon:   cdaemon stop"
echo "  Attach:        cdaemon hook <uuid-prefix>"
