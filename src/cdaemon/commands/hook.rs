use std::io::IsTerminal;
use std::process::{Command, Stdio};

use claude_rl_daemon::tmux::tmux_session_name;

use crate::state;

#[cfg(not(tarpaulin))]
pub fn run(uuid_or_prefix: &str) -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    let session_id = state::resolve_uuid(&daemon_state, uuid_or_prefix)?;
    let tmux_name = tmux_session_name(&session_id);

    let check = Command::new("tmux")
        .args(["has-session", "-t", &tmux_name])
        .output();

    match check {
        Err(e) => anyhow::bail!("tmux not found: {e}. Install with: brew install tmux"),
        Ok(out) if !out.status.success() => anyhow::bail!(
            "tmux session '{}' does not exist yet (daemon may not have resumed it)",
            tmux_name
        ),
        Ok(_) => {}
    }

    if std::io::stdout().is_terminal() {
        Command::new("tmux")
            .args(["attach", "-t", &tmux_name])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;
    } else {
        println!("tmux attach -t {tmux_name}");
    }

    Ok(())
}
