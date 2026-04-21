use chrono::{DateTime, Local, Utc, TimeZone};
use colored::Colorize;
use humantime;

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
        if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
            return Ok(dt);
        } else {
            return Err(anyhow::anyhow!("invalid epoch seconds: {}", secs));
        }
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

    println!("{} rescheduled pending resume for {} to {}", "✓".green(), &session_id[..8], new_dt.with_timezone(&Local).to_rfc3339());
    println!("Note: the daemon must be restarted to pick up this change immediately if it\'s already running.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_time;
    use chrono::Utc;

    #[test]
    fn parse_iso8601() {
        let dt = parse_time("2026-04-21T12:00:00Z").expect("parse rfc3339");
        assert_eq!(dt.to_rfc3339(), "2026-04-21T12:00:00+00:00");
    }

    #[test]
    fn parse_relative_plus() {
        let dt = parse_time("+2s").expect("parse +2s");
        assert!((dt - Utc::now()).num_seconds().abs() <= 5);
    }

    #[test]
    fn parse_in_minutes() {
        let dt = parse_time("in 1m").expect("parse in 1m");
        assert!((dt - Utc::now()).num_seconds() >= 50);
    }

    #[test]
    fn parse_epoch() {
        let dt = parse_time("1713638400").expect("parse epoch");
        assert_eq!(dt.timestamp(), 1713638400);
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_time("not a time").is_err());
    }
}
