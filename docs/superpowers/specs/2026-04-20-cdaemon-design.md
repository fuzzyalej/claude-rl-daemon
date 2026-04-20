# cdaemon CLI Tool — Design Spec

**Date:** 2026-04-20
**Status:** Approved

---

## Overview

`cdaemon` is a dedicated CLI management tool for `claude-rl-daemon`. It is a separate binary in the same Cargo workspace, sharing the `claude_rl_daemon` lib crate for state types. It communicates with the daemon exclusively through files — no IPC, no sockets.

**Goal:** Give users a single ergonomic command to inspect sessions, control the daemon service lifecycle, attach to tmux sessions, and diagnose issues.

---

## Architecture

### Binary target

New `[[bin]]` entry in `Cargo.toml`:

```toml
[[bin]]
name = "cdaemon"
path = "src/cdaemon/main.rs"
```

### File layout

```
src/
  cdaemon/
    main.rs              — clap app, dispatch to subcommands
    commands/
      status.rs          — daemon status + session summary
      sessions.rs        — full session table
      logs.rs            — tail daemon.log
      service.rs         — install/start/stop/uninstall via launchctl
      hook.rs            — tmux attach with TTY detection
      resume.rs          — manual resume
      cancel.rs          — remove session from state.json
      doctor.rs          — prerequisite checks
      completions.rs     — shell completion generation
    state.rs             — reads ~/.claude-daemon/state.json
    format.rs            — shared table/color helpers
```

### Communication model

`cdaemon` reads two files:

| File | Purpose |
|------|---------|
| `~/.claude-daemon/state.json` | Pending/completed session state written by daemon |
| `~/.claude-daemon/daemon.log` | Structured JSON log lines |

Service lifecycle commands (`install`, `start`, `stop`, `uninstall`) shell out to `launchctl` and operate on `~/Library/LaunchAgents/com.claudedaemon.plist`.

---

## New Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (derive + `clap_complete`) | CLI argument parsing + shell completions |
| `tabled` | Rich table output |
| `colored` | Terminal colors |

Existing deps (`chrono`, `serde`, `serde_json`, `anyhow`) are reused from the lib crate.

---

## Commands

### `cdaemon status`

Shows daemon health and a summary table of pending sessions.

**Output:**
```
Daemon    running  (pid 12345)
Sessions  2 pending, 0 completed today

 UUID                                   Reset At         CWD
 fc456884-d0b4-45f8-9d53-9a64dbc663d6   9:00 PM +0:15s   ~/Code/oje
 ab123456-d0b4-45f8-9d53-9a64dbc663d6   —                ~/Code/api
```

Daemon running state is determined by `launchctl list | grep com.claudedaemon`.

---

### `cdaemon sessions`

Full session history table with a STATUS column.

**Columns:** UUID, Status (pending / resumed / cancelled), Reset At, CWD

---

### `cdaemon logs [--follow] [-n N]`

Reads `~/.claude-daemon/daemon.log`. Default: last 50 lines.
`--follow` execs `tail -f` on the log file.

---

### `cdaemon install`

Runs `cargo build --release`, copies binary to `~/.local/bin/cdaemon` and `~/.local/bin/claude-rl-daemon`, installs plist to `~/Library/LaunchAgents/com.claudedaemon.plist` (substituting `__INSTALL_PATH__` and `__HOME__` via inline sed), then calls `launchctl load`. Idempotent: unloads existing plist before reloading.

---

### `cdaemon start`

Calls `launchctl load ~/Library/LaunchAgents/com.claudedaemon.plist`.

---

### `cdaemon stop`

Calls `launchctl unload ~/Library/LaunchAgents/com.claudedaemon.plist`.

---

### `cdaemon uninstall`

Stops the daemon, removes the plist, and removes `~/.local/bin/claude-rl-daemon`. Does NOT remove `~/.claude-daemon/` state/logs.

---

### `cdaemon hook <uuid>`

Attaches to the tmux session for a given session UUID.

- **If stdout is a TTY:** `exec tmux attach -t claude-rl-<8-char-prefix>`
- **If stdout is not a TTY:** prints the attach command

Accepts full UUID or 8-char prefix. Errors with exit code 1 if UUID not found or tmux session doesn't exist.

---

### `cdaemon resume <uuid>`

Manually triggers an immediate resume for a session UUID — spawns `tmux new-session -d` running `claude --resume <uuid>`. Accepts full UUID or 8-char prefix.

---

### `cdaemon cancel <uuid>`

Removes a session from the pending resumes in `state.json`. Accepts full UUID or 8-char prefix.

---

### `cdaemon doctor`

Checks all prerequisites and prints a checklist:

| Check | Pass | Fail hint |
|-------|------|-----------|
| `tmux` on PATH | ✓ | `brew install tmux` |
| daemon binary at `~/.local/bin/claude-rl-daemon` | ✓ | run `cdaemon install` |
| plist at `~/Library/LaunchAgents/com.claudedaemon.plist` | ✓ | run `cdaemon install` |
| launchd label `com.claudedaemon` loaded | ✓ | run `cdaemon start` |
| sessions dir `~/.claude/sessions/` exists | ✓ | open Claude Code once |
| state dir `~/.claude-daemon/` exists | ✓ | run `cdaemon start` |

---

### `cdaemon completions <bash|zsh|fish>`

Prints shell completion script to stdout using clap's `clap_complete` crate.

Usage: `cdaemon completions zsh > ~/.zsh/completions/_cdaemon`

---

## Output Style

- **Default:** Rich tables using `tabled`, colored status indicators via `colored`
  - Green = running / pending / success
  - Red = stopped / error
  - Yellow = warning
- **`--no-ui` flag:** Deferred to a future version

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Daemon not running | `status`/`sessions`/`logs` degrade gracefully; show "daemon stopped" |
| `state.json` missing | Treated as empty state, not an error |
| UUID not found | Clear error message, exit code 1 |
| No tmux installed | `hook` prints helpful error; `doctor` flags it |
| `install` already installed | Idempotent (unload + reload) |
| No permissions issue | All operations use `~/` paths, no sudo required |

**Exit codes:**
- `0` — success
- `1` — error
- `2` — not found (UUID lookup, etc.)

---

## Testing

- Unit tests for `state.rs` (parse state.json) and `format.rs` (table rendering)
- Integration test for `doctor` logic (mock filesystem checks)
- Integration test for `hook` TTY detection
- Manual smoke test: `cdaemon status`, `cdaemon sessions`, `cdaemon doctor` against a running daemon
