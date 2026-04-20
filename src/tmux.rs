use std::process::Command;

use tracing::{error, info};

pub fn tmux_session_name(session_id: &str) -> String {
    let prefix = &session_id[..8.min(session_id.len())];
    format!("claude-rl-{prefix}")
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

pub fn spawn_resume(session_id: &str, cwd: &std::path::Path) -> anyhow::Result<String> {
    let tmux_name = tmux_session_name(session_id);
    let args = build_tmux_args(&tmux_name, cwd, session_id);

    info!(session_id, tmux_session = tmux_name, "spawning tmux resume");

    let status = Command::new("tmux").args(&args).status()?;

    if !status.success() {
        error!(session_id, "tmux new-session failed");
        anyhow::bail!("tmux exited with status {status}");
    }

    info!(
        session_id,
        tmux_session = tmux_name,
        "resume launched — attach with: tmux attach -t {tmux_name}"
    );

    Ok(tmux_name)
}
