use chrono::{DateTime, Utc};
use colored::Colorize;
use humantime;
use std::time::Duration;

use crate::state;

fn parse_time(input: &str) -> anyhow::Result<DateTime<Utc>> {
    let s = input.trim();

    // Try RFC3339 / ISO8601
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Accept prefixes like "in 5m" or leading "+"
    let mut trimmed = s;
    if let Some(t) = trimmed.strip_prefix("in ") {
        trimmed = t.trim();
    }
    if let Some(t) = trimmed.strip_prefix('+') {
        trimmed = t.trim();
    }

    // Try human-friendly duration (e.g., "2h", "15m")
    if let Ok(std_dur) = humantime::parse_duration(trimmed) {
        let chrono_dur = chrono::Duration::from_std(std_dur)
            .map_err(|e| anyhow::anyhow!("invalid duration: {}", e))?;
        return Ok(Utc::now() + chrono_dur);
    }

    // Try parse as unix epoch seconds
    if let Ok(secs) = trimmed.parse::<i64>() {
        let naive = chrono::NaiveDateTime::from_timestamp_opt(secs, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid epoch seconds: {}", secs))?;
        return Ok(DateTime::<Utc>::from_utc(naive, Utc));
    }

    Err(anyhow::anyhow!("failed to parse time: {}", input))
}

pub fn run(uuid_or_prefix: &str, time_str: &str) -> anyhow::Result<()> {
    let mut daemon_state = state::load_state()?;
    let session_id = state::resolve_uuid(&daemon_state, uuid_or_prefix)?;

    let resume = daemon_state
        .pending
        .get_mut(&session_id)
        .ok_or_else(|| anyhow::anyhow!("session '{}' is not pending", session_id))?;

    let new_dt = parse_time(time_str)?;
    resume.reset_at = new_dt;

    state::save_state(&daemon_state)?;

    println!("{} rescheduled pending resume for {} to {}", "✓".green(), &session_id[..8], new_dt.to_rfc3339());
    println!("Note: the daemon must be restarted to pick up this change immediately if it\'s already running.");
    Ok(())
}
