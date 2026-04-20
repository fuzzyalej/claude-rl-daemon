use std::path::PathBuf;

use anyhow::Context;
use claude_rl_daemon::DaemonState;

pub fn state_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".claude-daemon/state.json")
}

pub fn log_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".claude-daemon/daemon.log")
}

pub fn plist_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join("Library/LaunchAgents/com.claudedaemon.plist")
}

pub fn daemon_bin_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".local/bin/claude-rl-daemon")
}

pub fn cdaemon_bin_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir not found")
        .join(".local/bin/cdaemon")
}

pub fn load_state() -> anyhow::Result<DaemonState> {
    DaemonState::load_from_path(&state_path()).context("failed to read state.json")
}

pub fn save_state(state: &DaemonState) -> anyhow::Result<()> {
    state.save_to_path(&state_path()).context("failed to write state.json")
}

/// Resolve a full UUID or 8-char prefix to a full session ID.
/// Returns error if not found or ambiguous.
pub fn resolve_uuid(state: &DaemonState, prefix_or_full: &str) -> anyhow::Result<String> {
    if state.pending.contains_key(prefix_or_full) {
        return Ok(prefix_or_full.to_string());
    }
    let matches: Vec<&str> = state
        .pending
        .keys()
        .filter(|k| k.starts_with(prefix_or_full))
        .map(|s| s.as_str())
        .collect();
    match matches.len() {
        0 => anyhow::bail!("no session found matching '{prefix_or_full}'"),
        1 => Ok(matches[0].to_string()),
        _ => anyhow::bail!(
            "ambiguous prefix '{}' matches {} sessions; use more characters",
            prefix_or_full,
            matches.len()
        ),
    }
}
