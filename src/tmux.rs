use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tracing::{error, info};

/// Resolves the tmux binary path.
///
/// LaunchAgents run with a stripped PATH that omits Homebrew, so we probe
/// known locations before falling back to the bare name.
pub fn find_tmux_binary() -> PathBuf {
    let candidates = [
        "/opt/homebrew/bin/tmux",  // Apple Silicon Homebrew
        "/usr/local/bin/tmux",     // Intel Homebrew
        "/usr/bin/tmux",
    ];
    for path in candidates {
        if std::path::Path::new(path).exists() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from("tmux")
}

pub fn tmux_session_name(session_id: &str) -> String {
    let prefix = &session_id[..8.min(session_id.len())];
    format!("claude-rl-{prefix}")
}

pub fn build_send_keys_args(tmux_name: &str) -> Vec<String> {
    vec![
        "send-keys".into(),
        "-t".into(),
        tmux_name.to_string(),
        "continue".into(),
        "Enter".into(),
    ]
}

pub fn build_tmux_args(tmux_name: &str, cwd: &std::path::Path, session_id: &str) -> Vec<String> {
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        tmux_name.to_string(),
        "-c".into(),
        cwd.to_string_lossy().to_string(),
        format!("claude --resume {session_id}"),
    ]
}

#[cfg(not(tarpaulin))]
pub fn spawn_resume(session_id: &str, cwd: &std::path::Path) -> anyhow::Result<String> {
    let tmux_name = tmux_session_name(session_id);
    let args = build_tmux_args(&tmux_name, cwd, session_id);
    let tmux_bin = find_tmux_binary();

    info!(session_id, tmux_session = tmux_name, tmux_bin = %tmux_bin.display(), "spawning tmux resume");

    let status = Command::new(&tmux_bin).args(&args).status()?;

    if !status.success() {
        error!(session_id, "tmux new-session failed");
        anyhow::bail!("tmux exited with status {status}");
    }

    info!(
        session_id,
        tmux_session = tmux_name,
        "resume launched — attach with: tmux attach -t {tmux_name}"
    );

    std::thread::sleep(Duration::from_secs(5));
    let sk_status = Command::new(&tmux_bin)
        .args(build_send_keys_args(&tmux_name))
        .status()?;
    if !sk_status.success() {
        anyhow::bail!("tmux send-keys exited with status {sk_status}");
    }
    info!(session_id, tmux_session = tmux_name, "sent 'continue' to session");

    Ok(tmux_name)
}

/// Stub used by tarpaulin runs so handle_change can compile without a real tmux.
#[cfg(tarpaulin)]
pub fn spawn_resume(session_id: &str, _cwd: &std::path::Path) -> anyhow::Result<String> {
    Ok(tmux_session_name(session_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_tmux_binary_returns_existing_path_or_fallback() {
        let bin = find_tmux_binary();
        // Either it found an absolute path that actually exists, or fell back to bare "tmux"
        if bin.is_absolute() {
            assert!(bin.exists(), "find_tmux_binary returned absolute path that doesn't exist: {}", bin.display());
        } else {
            assert_eq!(bin, PathBuf::from("tmux"));
        }
    }

    #[test]
    fn tmux_session_name_uses_first_8_chars() {
        let name = tmux_session_name("fc456884-d0b4-45f8-9d53-9a64dbc663d6");
        assert_eq!(name, "claude-rl-fc456884");
    }

    #[test]
    fn tmux_session_name_short_id() {
        let name = tmux_session_name("abc");
        assert_eq!(name, "claude-rl-abc");
    }

    #[test]
    fn build_send_keys_args_structure() {
        let args = build_send_keys_args("claude-rl-abc12345");
        assert_eq!(args[0], "send-keys");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "claude-rl-abc12345");
        assert_eq!(args[3], "continue");
        assert_eq!(args[4], "Enter");
    }

    #[test]
    fn build_tmux_args_structure() {
        let args = build_tmux_args("claude-rl-abc12345", std::path::Path::new("/tmp"), "abc12345-full-id");
        assert_eq!(args[0], "new-session");
        assert_eq!(args[3], "claude-rl-abc12345");
        assert_eq!(args[5], "/tmp");
        assert!(args[6].contains("--resume abc12345-full-id"));
    }
}
