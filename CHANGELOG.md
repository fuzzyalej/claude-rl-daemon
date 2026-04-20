# Changelog

All notable changes to this project will be documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-20

### Added
- File watcher using `notify` (macOS FSEvents) to monitor `~/.claude/sessions/` and active session JSONL files
- Rate-limit detector that identifies Claude Code's synthetic `assistant` messages with `"error":"rate_limit"` / `"apiErrorStatus":429`
- Reset-time parser for Claude Code's natural-language format: `"resets 9pm (Europe/Madrid)"` using `chrono-tz` for accurate IANA timezone conversion
- Fallback chain: ISO timestamp → `retry-after` seconds → 5-minute default
- Scheduler with deduplication — never fires two resumes for the same session UUID
- State persistence to `~/.claude-daemon/state.json` — pending resumes survive daemon restarts
- Automatic resume via `tmux new-session -d` running `claude --resume <uuid>`
- Structured JSON logging (`tracing` + `tracing-subscriber`) to `~/.claude-daemon/daemon.log`
- macOS launchd deployment (`deploy/install.sh`) — starts at login, restarts on crash
- Phase 1 forensics scripts (`scripts/phase1-before.sh`, `scripts/phase1-after.sh`) for inspecting Claude Code's local storage format
