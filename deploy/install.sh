#!/usr/bin/env bash
set -euo pipefail

BINARY="target/release/claude-rl-daemon"
INSTALL_PATH="/usr/local/bin/claude-rl-daemon"
PLIST_SRC="deploy/com.claudedaemon.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.claudedaemon.plist"

echo "Building release binary..."
cargo build --release

echo "Installing binary to $INSTALL_PATH"
cp "$BINARY" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

echo "Installing launchd plist..."
mkdir -p "$HOME/Library/LaunchAgents"
cp "$PLIST_SRC" "$PLIST_DST"

# Unload if already loaded (idempotent)
launchctl unload "$PLIST_DST" 2>/dev/null || true
launchctl load "$PLIST_DST"

echo "Done. Daemon is running."
echo ""
echo "Useful commands:"
echo "  Check status:  launchctl list | grep claudedaemon"
echo "  View logs:     tail -f /tmp/claude-rl-daemon.log"
echo "  Stop daemon:   launchctl unload $PLIST_DST"
echo "  Attach resume: tmux ls | grep claude-rl"
echo "                 tmux attach -t claude-rl-<uuid-prefix>"
