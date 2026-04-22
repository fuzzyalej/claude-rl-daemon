use claude_rl_daemon::DaemonState;
use colored::Colorize;

use crate::state;

#[cfg(not(tarpaulin))]
pub fn run(uuid_or_prefix: &str) -> anyhow::Result<()> {
    let mut daemon_state = state::load_state()?;
    execute(&mut daemon_state, uuid_or_prefix)?;
    state::save_state(&daemon_state)
}

pub fn execute(daemon_state: &mut DaemonState, uuid_or_prefix: &str) -> anyhow::Result<()> {
    let session_id = state::resolve_uuid(daemon_state, uuid_or_prefix)?;
    daemon_state.pending.remove(&session_id);
    println!("{} cancelled pending resume for {}", "✓".green(), &session_id[..8]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use claude_rl_daemon::PendingResume;

    fn state_with(id: &str) -> DaemonState {
        let mut s = DaemonState::default();
        s.pending.insert(id.to_string(), PendingResume {
            session_id: id.to_string(),
            reset_at: Utc::now(),
            cwd: None,
        });
        s
    }

    #[test]
    fn removes_by_full_uuid() {
        let id = "abc12345-0000-0000-0000-000000000000";
        let mut s = state_with(id);
        execute(&mut s, id).unwrap();
        assert!(!s.pending.contains_key(id));
    }

    #[test]
    fn removes_by_prefix() {
        let id = "abc12345-0000-0000-0000-000000000000";
        let mut s = state_with(id);
        execute(&mut s, "abc12345").unwrap();
        assert!(s.pending.is_empty());
    }

    #[test]
    fn errors_on_unknown_session() {
        let mut s = DaemonState::default();
        assert!(execute(&mut s, "notfound").is_err());
    }
}
