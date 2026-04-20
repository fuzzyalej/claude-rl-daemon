# claude-rl-daemon Architecture

## Detection pipeline

1. `watcher.rs` uses `notify` (FSEvents on macOS) to watch:
   - `~/.claude/sessions/` — for new session PID files (new `<pid>.json` → discover its JSONL)
   - `~/.claude/projects/<project>/<uuid>.jsonl` — for new appended lines

2. `detector.rs` scans new JSONL lines with compiled `OnceLock<Regex>` patterns.
   Returns a `RateLimitEvent { session_id, reset_at, cwd }` or `None`.

3. `scheduler.rs` deduplicates by `session_id` and persists pending resumes
   to `~/.claude-daemon/state.json` so the daemon survives restarts.

4. `tmux.rs` calls:
   ```
   tmux new-session -d -s claude-rl-<prefix> -c <cwd> "claude --resume <uuid>"
   ```
   Attach with: `tmux attach -t claude-rl-<prefix>`

## Tuning rate-limit patterns

After running `scripts/phase1-before.sh`, triggering a rate limit, then
`scripts/phase1-after.sh`, update `RATE_LIMIT_RE` in `src/detector.rs`
with the exact strings Claude Code emits.

Current patterns cover:
- `rate limit` / `rate-limit`
- `too many requests`
- `quota exceeded` / `quota exhausted`
- `usage limit`
- `overloaded`
- `retry after`
- HTTP `429`

## Logs

```bash
RUST_LOG=debug ./target/release/claude-rl-daemon
```

JSON-formatted log lines, readable with `jq`:
```bash
tail -f /tmp/claude-rl-daemon.log | jq .
```

## State file

`~/.claude-daemon/state.json` — human-readable JSON, safe to delete to reset all pending resumes.

## Install

```bash
bash deploy/install.sh
```

This builds a release binary, installs to `/usr/local/bin/claude-rl-daemon`,
and registers a launchd agent that starts at login and restarts on crash.
