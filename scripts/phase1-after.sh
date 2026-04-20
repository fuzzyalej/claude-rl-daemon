#!/usr/bin/env bash
# Run IMMEDIATELY after Claude Code halts with a rate limit.
set -euo pipefail

SNAP_DIR="/tmp/claude-rl-snapshot"

echo "=== Sessions directory AFTER ==="
ls -la ~/.claude/sessions/ 2>/dev/null

echo -e "\n=== Session diff (removed = process exited) ==="
diff "$SNAP_DIR/sessions-before.txt" <(ls -la ~/.claude/sessions/ 2>/dev/null) || true

echo -e "\n=== Last 10 lines of active JSONL ==="
JSONL=$(cat "$SNAP_DIR/active-jsonl.txt" 2>/dev/null)
if [ -n "$JSONL" ] && [ -f "$JSONL" ]; then
  tail -n10 "$JSONL" | python3 -c \
    "import sys,json; [print(json.dumps(json.loads(l),indent=2)) for l in sys.stdin if l.strip()]"
fi

echo -e "\n=== Any debug log? ==="
find /tmp -name "claude-debug*.log" -newer "$SNAP_DIR/sessions-before.txt" 2>/dev/null \
  | while read -r f; do
      echo "--- $f ---"
      grep -i "rate\|limit\|429\|quota\|retry\|reset\|overload" "$f" | tail -20
    done

echo -e "\n=== WHAT TO LOOK FOR ==="
echo "1. What 'type' and 'subtype' did the final JSONL message have?"
echo "2. Is there a 'resetAt', 'retryAfter', or timestamp field in that message?"
echo "3. Did the session .json file in ~/.claude/sessions/ disappear (process exited)?"
echo "4. Share this output so detector.rs patterns can be finalized."
