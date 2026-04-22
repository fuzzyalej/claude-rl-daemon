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

pub fn shorten_home(path: String) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path.starts_with(home_str.as_ref()) {
            return format!("~{}", &path[home_str.len()..]);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn pending_resume(id: &str, cwd: Option<&str>) -> PendingResume {
        PendingResume {
            session_id: id.to_string(),
            reset_at: Utc::now() + Duration::seconds(120),
            cwd: cwd.map(std::path::PathBuf::from),
        }
    }

    #[test]
    fn color_status_known_labels() {
        assert!(color_status("pending").contains("pending"));
        assert!(color_status("resumed").contains("resumed"));
        assert!(color_status("cancelled").contains("cancelled"));
        assert!(color_status("running").contains("running"));
        assert!(color_status("stopped").contains("stopped"));
    }

    #[test]
    fn color_status_unknown_passthrough() {
        assert_eq!(color_status("unknown-label"), "unknown-label");
    }

    #[test]
    fn format_reset_at_future_includes_countdown() {
        let future = Utc::now() + Duration::seconds(90);
        let s = format_reset_at(future);
        assert!(s.contains("+"), "expected '+Ns' suffix in '{s}'");
    }

    #[test]
    fn format_reset_at_past_no_countdown() {
        let past = Utc::now() - Duration::seconds(10);
        let s = format_reset_at(past);
        assert!(!s.contains('+'), "expected no '+Ns' in past time '{s}'");
    }

    #[test]
    fn format_sessions_empty_returns_no_sessions() {
        let s = format_sessions(&[], "pending");
        assert!(s.contains("No sessions"));
    }

    #[test]
    fn format_sessions_returns_table_with_uuid() {
        let r = pending_resume("my-session-id", None);
        let s = format_sessions(&[r], "pending");
        assert!(s.contains("my-session-id"), "table should contain session id");
    }

    #[test]
    fn session_row_no_cwd_uses_dash() {
        let r = pending_resume("abc", None);
        let row = session_row(&r, "pending");
        assert_eq!(row.cwd, "—");
    }

    #[test]
    fn session_row_cwd_outside_home_unchanged() {
        let r = pending_resume("abc", Some("/tmp/project"));
        let row = session_row(&r, "pending");
        assert_eq!(row.cwd, "/tmp/project");
    }

    #[test]
    fn shorten_home_replaces_prefix() {
        if let Some(home) = dirs::home_dir() {
            let full = format!("{}/Code/project", home.display());
            let short = shorten_home(full);
            assert!(short.starts_with("~/"), "expected ~/... but got '{short}'");
        }
    }

    #[test]
    fn shorten_home_no_match_unchanged() {
        let s = shorten_home("/tmp/other".to_string());
        assert_eq!(s, "/tmp/other");
    }
}
