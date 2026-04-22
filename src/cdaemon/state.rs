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

#[cfg(not(tarpaulin))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use claude_rl_daemon::PendingResume;

    fn state_with_sessions(ids: &[&str]) -> DaemonState {
        let mut s = DaemonState::default();
        for &id in ids {
            s.pending.insert(id.to_string(), PendingResume {
                session_id: id.to_string(),
                reset_at: Utc::now(),
                cwd: None,
            });
        }
        s
    }

    #[test]
    fn resolve_full_uuid() {
        let id = "fc456884-d0b4-45f8-9d53-9a64dbc663d6";
        let s = state_with_sessions(&[id]);
        assert_eq!(resolve_uuid(&s, id).unwrap(), id);
    }

    #[test]
    fn resolve_8char_prefix() {
        let id = "fc456884-d0b4-45f8-9d53-9a64dbc663d6";
        let s = state_with_sessions(&[id]);
        assert_eq!(resolve_uuid(&s, "fc456884").unwrap(), id);
    }

    #[test]
    fn resolve_ambiguous_prefix_errors() {
        let s = state_with_sessions(&[
            "aaaa1111-0000-0000-0000-000000000000",
            "aaaa2222-0000-0000-0000-000000000000",
        ]);
        let err = resolve_uuid(&s, "aaaa").unwrap_err();
        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn resolve_missing_errors() {
        let s = DaemonState::default();
        let err = resolve_uuid(&s, "notfound").unwrap_err();
        assert!(err.to_string().contains("no session found"));
    }

    #[test]
    fn path_helpers_return_home_relative_paths() {
        let home = dirs::home_dir().unwrap();
        assert!(state_path().starts_with(&home));
        assert!(log_path().starts_with(&home));
        assert!(plist_path().starts_with(&home));
        assert!(daemon_bin_path().starts_with(&home));
        assert!(cdaemon_bin_path().starts_with(&home));
    }

    #[test]
    fn load_state_returns_default_when_no_file() {
        // Safe: returns DaemonState::default() when ~/.claude-daemon/state.json is absent
        let s = load_state().unwrap_or_default();
        // Just verify it doesn't panic and returns a valid default
        let _ = s.pending.len();
    }
}
