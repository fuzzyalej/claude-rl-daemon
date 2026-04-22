use claude_rl_daemon::tmux::spawn_resume;

use crate::state;

#[cfg(not(tarpaulin))]
pub fn run(uuid_or_prefix: &str) -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    let session_id = state::resolve_uuid(&daemon_state, uuid_or_prefix)?;

    let resume = daemon_state
        .pending
        .get(&session_id)
        .ok_or_else(|| anyhow::anyhow!("session '{}' is not pending", session_id))?;

    let cwd = resume.cwd.clone()
        .unwrap_or_else(|| dirs::home_dir().expect("home dir not found"));

    let tmux_name = spawn_resume(&session_id, &cwd)?;
    println!("Resumed in tmux session: {tmux_name}");
    println!("Attach with: tmux attach -t {tmux_name}");
    Ok(())
}
