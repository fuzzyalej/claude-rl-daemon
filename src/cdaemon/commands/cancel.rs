use colored::Colorize;

use crate::state;

pub fn run(uuid_or_prefix: &str) -> anyhow::Result<()> {
    let mut daemon_state = state::load_state()?;
    let session_id = state::resolve_uuid(&daemon_state, uuid_or_prefix)?;

    daemon_state.pending.remove(&session_id);
    state::save_state(&daemon_state)?;

    println!("{} cancelled pending resume for {}", "✓".green(), &session_id[..8]);
    Ok(())
}
