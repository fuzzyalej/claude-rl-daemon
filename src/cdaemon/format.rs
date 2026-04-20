use chrono::{DateTime, Local, Utc};
use claude_rl_daemon::PendingResume;
use colored::Colorize;
use tabled::{Table, Tabled};

#[derive(Tabled)]
pub struct SessionRow {
    #[tabled(rename = "UUID")]
    pub uuid: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Reset At")]
    pub reset_at: String,
    #[tabled(rename = "CWD")]
    pub cwd: String,
}

pub fn format_sessions(resumes: &[PendingResume], status_label: &str) -> String {
    if resumes.is_empty() {
        return "No sessions.".dimmed().to_string();
    }
    let rows: Vec<SessionRow> = resumes.iter().map(|r| session_row(r, status_label)).collect();
    Table::new(rows).to_string()
}

pub fn session_row(r: &PendingResume, status_label: &str) -> SessionRow {
    SessionRow {
        uuid: r.session_id.clone(),
        status: color_status(status_label),
        reset_at: format_reset_at(r.reset_at),
        cwd: r.cwd
            .as_ref()
            .map(|p| shorten_home(p.to_string_lossy().to_string()))
            .unwrap_or_else(|| "—".to_string()),
    }
}

pub fn color_status(s: &str) -> String {
    match s {
        "pending"   => s.yellow().to_string(),
        "resumed"   => s.green().to_string(),
        "cancelled" => s.red().to_string(),
        "running"   => s.green().to_string(),
        "stopped"   => s.red().to_string(),
        _           => s.to_string(),
    }
}

pub fn format_reset_at(dt: DateTime<Utc>) -> String {
    let local: DateTime<Local> = dt.into();
    let secs = dt.timestamp() - Utc::now().timestamp();
    if secs > 0 {
        format!("{} (+{}s)", local.format("%I:%M %p"), secs)
    } else {
        local.format("%I:%M %p").to_string()
    }
}

fn shorten_home(path: String) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path.starts_with(home_str.as_ref()) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path
}
