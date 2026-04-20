use colored::Colorize;

use crate::{format, state};

pub fn run() -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    let pending: Vec<_> = daemon_state.pending.values().cloned().collect();

    let launchctl_out = std::process::Command::new("launchctl")
        .args(["list", "com.claudedaemon"])
        .output();

    let (daemon_label, pid_label) = match launchctl_out {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let pid = stdout
                .lines()
                .find(|l| !l.starts_with("PID"))
                .and_then(|l| l.split_whitespace().next())
                .filter(|&p| p != "-")
                .map(|p| format!("(pid {p})"))
                .unwrap_or_default();
            ("running".green().to_string(), pid)
        }
        _ => ("stopped".red().to_string(), String::new()),
    };

    println!("Daemon    {daemon_label}  {pid_label}");
    println!(
        "Sessions  {} pending, {} completed",
        pending.len(),
        daemon_state.completed.len()
    );

    if !pending.is_empty() {
        println!();
        println!("{}", format::format_sessions(&pending, "pending"));
    }

    Ok(())
}
