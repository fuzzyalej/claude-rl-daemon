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
echo "  Check status:  launchctl list | grep claudedaemon"
echo "  View logs:     tail -f $HOME/.claude-daemon/daemon.log"
echo "  Stop daemon:   launchctl unload $PLIST_DST"
echo "  Attach resume: tmux ls | grep claude-rl"
echo "                 tmux attach -t claude-rl-<uuid-prefix>"
