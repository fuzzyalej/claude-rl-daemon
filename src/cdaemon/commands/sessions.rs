use claude_rl_daemon::DaemonState;
use colored::Colorize;
use tabled::{Table, Tabled};

use crate::{format, state};

#[derive(Tabled)]
struct SessionRow {
    #[tabled(rename = "#")]
    index: String,
    #[tabled(rename = "UUID")]
    uuid: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Reset At")]
    reset_at: String,
    #[tabled(rename = "CWD")]
    cwd: String,
}

#[cfg(not(tarpaulin))]
pub fn run() -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    print_sessions(&daemon_state);
    Ok(())
}

pub fn print_sessions(daemon_state: &DaemonState) {
    let mut rows: Vec<SessionRow> = Vec::new();

    let pending: Vec<claude_rl_daemon::PendingResume> =
        daemon_state.pending.values().cloned().collect();
    for (i, resume) in format::sorted_pending(&pending).iter().enumerate() {
        let r = format::session_row(resume, "pending", i + 1);
        rows.push(SessionRow { index: r.index, uuid: r.uuid, status: r.status, reset_at: r.reset_at, cwd: r.cwd });
    }

    for id in &daemon_state.completed {
        rows.push(SessionRow {
            index: "—".to_string(),
            uuid: id.clone(),
            status: format::color_status("resumed"),
            reset_at: "—".to_string(),
            cwd: "—".to_string(),
        });
    }

    if rows.is_empty() {
        println!("{}", "No sessions recorded.".dimmed());
        return;
    }

    println!("{}", Table::new(rows));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use claude_rl_daemon::PendingResume;

    #[test]
    fn empty_state_prints_no_sessions() {
        let s = DaemonState::default();
        // Should not panic; output goes to stdout which we can't easily capture, so just verify no crash
        print_sessions(&s);
    }

    #[test]
    fn state_with_pending_and_completed() {
        let mut s = DaemonState::default();
        s.pending.insert("abc".to_string(), PendingResume {
            session_id: "abc".to_string(),
            reset_at: Utc::now(),
            cwd: None,
        });
        s.completed.insert("def".to_string());
        print_sessions(&s);
    }
}
