# claude-rl-daemon Architecture

## Detection pipeline

```
FSEvents (notify)
   └── watcher.rs
         ├── sessions dir: new <pid>.json  → queue JSONL for watching
         └── JSONL file changed           → read_new_lines() → detect_rate_limit()
                                                                      │
                                                               detector.rs
                                                                      │
                                                               RateLimitEvent
                                                                      │
                                                              scheduler.rs
                                                          (deduplicate + persist)
                                                                      │
                                                            resume_after() (watcher.rs)
                                                               sleep(delay)
                                                                      │
                                                               tmux.rs
                                                        spawn_resume() → tmux new-session
```

## Module responsibilities

### `watcher.rs`
- `run()` — top-level async loop; watches FSEvents via `notify` crate
- `handle_change()` — dispatches on file extension: `.json` (session PID) → queue JSONL; `.jsonl` → detect rate limits
- `read_new_lines()` — incremental file read using a seek-offset map (tail-follow pattern)
- `discover_active_jsonls()` — on startup, finds JSONL paths for already-open sessions
- `resume_after()` — awaits `reset_at`, then calls `spawn_resume`; exposed as `pub` for testing

### `detector.rs`
- Parses raw JSONL lines with `OnceLock<Regex>` for zero-allocation hot path
- Reset time extraction priority:
  1. `"resets Xpm (Timezone)"` in message text (rolls forward to tomorrow if passed)
  2. ISO8601 timestamp with `Z` suffix in the raw line
  3. `Retry-After: N` seconds
  4. Default fallback: 300s + 15s buffer

### `scheduler.rs`
- Deduplicates by `session_id` — ignores both pending and completed sessions
- Persists `DaemonState` (HashMap + HashSet) to `~/.claude-daemon/state.json` on every change
- On restart, `Scheduler::new()` reloads state and `restore_pending_resumes()` re-arms all timers

### `tmux.rs`
- `find_tmux_binary()` — probes known Homebrew locations before falling back to bare `tmux` (LaunchAgents run with a stripped PATH)
- `tmux_session_name()` — `claude-rl-<uuid_prefix_8chars>`
- `build_tmux_args()` — pure function for testing without exec
- `spawn_resume()` — runs `tmux new-session -d -s <name> -c <cwd> "claude --resume <uuid>"`
- All three pure functions are exported from the library crate for use by `cdaemon hook`

### `notify.rs`
- macOS: `osascript` for native notifications
- Other platforms: log-only no-op

### `cdaemon/`
- `state.rs` — path helpers + `resolve_uuid()` (full UUID, 8-char prefix, or 1-based row index matching `cdaemon sessions` order)
- `format.rs` — pure formatting functions for table output
- `commands/` — one module per CLI subcommand; business logic extracted to testable pure functions

## Testability strategy

System-interaction code (tmux, launchctl, osascript) is excluded from coverage via `#[cfg(not(tarpaulin))]`. These functions are compiled out when tarpaulin runs, and stubs are provided where necessary (e.g., `spawn_resume` returns a fake session name without invoking tmux). This approach avoids polluting production types with test-only interfaces while still achieving 97%+ line coverage.

The two `main()` entry points and the tmux-binary fallback path are not coverable without OS-level mocking and are excluded from the coverage target.

## Rate-limit detection patterns

Current patterns in `detector.rs` match:
- `"error": "rate_limit"` field
- `"apiErrorStatus": 429`
- `"isApiErrorMessage": true`

Reset time is parsed from the message text, ISO timestamps, or `Retry-After` headers.

## State file

`~/.claude-daemon/state.json` — human-readable JSON, safe to delete to reset all pending resumes.

## Logs

```bash
RUST_LOG=debug ./target/release/claude-rl-daemon
tail -f ~/.claude-daemon/daemon.log | jq .
```

## Install

```bash
cdaemon install   # preferred
# or directly:
bash deploy/install.sh
```

Builds release binaries, installs to `~/.local/bin/`, and registers a launchd agent.
