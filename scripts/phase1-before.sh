#!/usr/bin/env bash
# Run BEFORE triggering a rate limit. Snapshots current session state.
set -euo pipefail

SNAP_DIR="/tmp/claude-rl-snapshot"
mkdir -p "$SNAP_DIR"

echo "=== Active Sessions ==="
ls -la ~/.claude/sessions/ 2>/dev/null | tee "$SNAP_DIR/sessions-before.txt"

echo -e "\n=== Session File Contents ==="
for f in ~/.claude/sessions/*.json; do
  [ -f "$f" ] || continue
  echo "--- $f ---"
  cat "$f"
  echo
done | tee "$SNAP_DIR/session-contents-before.txt"

echo -e "\n=== Most Recent JSONL (last 3 lines) ==="
RECENT=$(find ~/.claude/projects -name "*.jsonl" -not -path "*/subagents/*" \
  -newer ~/.claude/stats-cache.json 2>/dev/null | head -1)
if [ -n "$RECENT" ]; then
  echo "Watching: $RECENT"
  echo "$RECENT" > "$SNAP_DIR/active-jsonl.txt"
  tail -n3 "$RECENT" | python3 -c \
    "import sys,json; [print(json.dumps(json.loads(l),indent=2)) for l in sys.stdin if l.strip()]"
fi

echo -e "\nSnapshot saved to $SNAP_DIR — now trigger a rate limit, then run phase1-after.sh"
