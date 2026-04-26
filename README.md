# claude-rl-daemon

A background Rust daemon that watches Claude Code sessions and automatically resumes them in a detached tmux window after an API rate limit resets.

## How it works

1. Watches `~/.claude/sessions/` for active Claude Code processes
2. Tails each session's JSONL file for rate-limit error messages
3. Extracts the reset timestamp and waits (plus a 15-second buffer)
4. Spawns `claude --resume <uuid>` in a detached tmux session

## Install

```bash
bash deploy/install.sh
```

Builds release binaries, installs `claude-rl-daemon` and `cdaemon` to `~/.local/bin/`, and registers a launchd agent that starts at login and restarts on crash.

## Management CLI (cdaemon)

`cdaemon` is the management interface for the daemon.

### Interactive TUI

Running `cdaemon` with no arguments opens an interactive dashboard:

```bash
cdaemon
```

The TUI shows:
- **Status bar** — daemon running state and last refresh time
- **Sessions table** — pending resumes with UUID, project directory, and countdown
- **Logs panel** — last 200 lines from the daemon log

**Keybindings:**

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate sessions |
| `x` | Cancel selected resume |
| `e` | Resume selected session now |
| `s` | Reschedule selected session (enter new time, e.g. `+2h`) |
| `h` | Attach to selected session's tmux window |
| `d` | Run doctor check (overlay) |
| `l` | Expand logs fullscreen |
| `r` | Force refresh |
| `Tab` | Toggle focus (sessions / logs) |
| `q` / `Esc` / `Ctrl-C` | Quit |

### Quick reference

| Command | Description |
|---------|-------------|
| `cdaemon status` | Daemon health + pending sessions |
| `cdaemon sessions` | Full session history |
| `cdaemon logs` | View last 50 log lines |
| `cdaemon logs --follow` | Tail live log output |
| `cdaemon hook <uuid>` | Attach to a session's tmux window |
| `cdaemon resume <uuid>` | Manually trigger a resume now |
| `cdaemon reschedule <uuid> <time>` | Reschedule a pending resume (ISO8601 or relative, e.g. "+2h", "in 10m") |
| `cdaemon cancel <uuid>` | Cancel a pending resume |
| `cdaemon doctor` | Check all prerequisites |
| `cdaemon install` | Build + install daemon + configure launchd |
| `cdaemon start` / `cdaemon stop` | Start or stop the daemon |
| `cdaemon completions zsh` | Print zsh completion script |

Commands that take a UUID accept a full UUID, the first 8 characters, or the 1-based row index shown by `cdaemon sessions` (e.g. `cdaemon cancel 1`).

### Shell completions

```bash
# zsh
cdaemon completions zsh > ~/.zsh/completions/_cdaemon

# bash
cdaemon completions bash >> ~/.bashrc
```

## Logs

Logs are written as JSON to `~/.claude-daemon/`:

| File | Contents |
|------|----------|
| `~/.claude-daemon/daemon.log` | Info/debug output (rate limits detected, resumes scheduled) |
| `~/.claude-daemon/daemon.err` | Errors (tmux failures, file access issues) |
| `~/.claude-daemon/state.json` | Pending/completed resume state (human-readable, safe to delete) |

### Viewing logs

```bash
# Live structured log stream
tail -f ~/.claude-daemon/daemon.log | jq .

# Show only rate-limit and resume events
tail -f ~/.claude-daemon/daemon.log | jq 'select(.fields.message | test("rate limit|resume|scheduled"))'

# Check daemon status (launchd)
launchctl list | grep claudedaemon
```

### Log verbosity

Set `RUST_LOG` in the plist (default: `info`). Options: `error`, `warn`, `info`, `debug`.

```bash
# Edit the plist before installing
nano deploy/com.claudedaemon.plist
# Change the RUST_LOG value, then re-run: bash deploy/install.sh
```

## Attaching to a resumed session

When the daemon resumes a session it logs the tmux session name:

```json
{"level":"INFO","fields":{"message":"resume spawned","tmux_session":"claude-rl-fc456884"}}
```

Attach with:

```bash
tmux ls | grep claude-rl          # list all daemon-spawned sessions
tmux attach -t claude-rl-fc456884 # attach to a specific one
```

## Manage the daemon

```bash
# Stop / Start (preferred)
cdaemon stop
cdaemon start

# Or directly via launchctl
launchctl unload ~/Library/LaunchAgents/com.claudedaemon.plist
launchctl load ~/Library/LaunchAgents/com.claudedaemon.plist

# Uninstall
cdaemon uninstall
# or manually:
launchctl unload ~/Library/LaunchAgents/com.claudedaemon.plist
rm ~/Library/LaunchAgents/com.claudedaemon.plist
rm ~/.local/bin/claude-rl-daemon
rm ~/.local/bin/cdaemon
```

## Run manually (no launchd)

```bash
RUST_LOG=info ./target/release/claude-rl-daemon
```

Logs go to stdout only when running manually.

## Tuning rate-limit detection

After hitting a real rate limit, run the forensics scripts to capture the exact JSONL format:

```bash
bash scripts/phase1-before.sh   # snapshot before
# ... trigger rate limit ...
bash scripts/phase1-after.sh    # capture the diff
```

Then update `RATE_LIMIT_RE` in `src/detector.rs` with the exact strings Claude Code emits and rebuild.

## Architecture

See [docs/architecture.md](docs/architecture.md) for a full pipeline description.

## Notifications

The daemon emits native notifications on macOS when a resume is scheduled and when a session is resumed (or fails to resume). This uses the system "osascript" command so there are no extra dependencies. On non-macOS platforms notifications are a no-op and are logged at info level.
