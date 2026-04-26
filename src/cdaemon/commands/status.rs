use claude_rl_daemon::DaemonState;
use colored::Colorize;

use crate::{format, state};

#[cfg(not(tarpaulin))]
pub fn run() -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    let daemon_status = query_launchctl_status();
    let claude_dir = dirs::home_dir().expect("home dir").join(".claude");
    let active_count = claude_rl_daemon::watcher::discover_active_jsonls(&claude_dir).len();
    print_status(&daemon_state, &daemon_status, active_count);
    Ok(())
}

pub struct DaemonStatus {
    pub label: String,
    pub pid: String,
}

#[cfg(not(tarpaulin))]
pub fn query_launchctl_status() -> DaemonStatus {
    let launchctl_out = std::process::Command::new("launchctl")
        .args(["list", "com.claudedaemon"])
        .output();

    match launchctl_out {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let pid = stdout
                .lines()
                .find(|l| !l.starts_with("PID"))
                .and_then(|l| l.split_whitespace().next())
                .filter(|&p| p != "-")
                .map(|p| format!("(pid {p})"))
                .unwrap_or_default();
            DaemonStatus { label: "running".green().to_string(), pid }
        }
        _ => DaemonStatus { label: "stopped".red().to_string(), pid: String::new() },
    }
}

pub fn print_status(daemon_state: &DaemonState, status: &DaemonStatus, active_count: usize) {
    let pending: Vec<_> = daemon_state.pending.values().cloned().collect();

    println!("Daemon    {}  {}", status.label, status.pid);
    println!(
        "Sessions  {} active, {} pending, {} completed",
        active_count,
        pending.len(),
        daemon_state.completed.len()
    );

    if !pending.is_empty() {
        println!();
        println!("Pending Resumes:");
        println!("{}", format::format_sessions(&pending, "pending"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use claude_rl_daemon::PendingResume;

    fn stopped() -> DaemonStatus {
        DaemonStatus { label: "stopped".to_string(), pid: String::new() }
    }

    #[test]
    fn prints_empty_state() {
        print_status(&DaemonState::default(), &stopped(), 0);
    }

    #[test]
    fn prints_pending_sessions() {
        let mut s = DaemonState::default();
        s.pending.insert("xyz".to_string(), PendingResume {
            session_id: "xyz".to_string(),
            reset_at: Utc::now(),
            cwd: None,
        });
        print_status(&s, &stopped(), 1);
    }

    #[test]
    fn prints_completed_sessions() {
        let mut s = DaemonState::default();
        s.completed.insert("done-session".to_string());
        print_status(&s, &stopped(), 0);
    }
}
