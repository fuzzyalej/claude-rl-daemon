use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::{DateTime, Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use regex::Regex;
use serde_json::Value;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct RateLimitEvent {
    pub session_id: String,
    pub reset_at: DateTime<Utc>,
    pub cwd: Option<PathBuf>,
}

// ── Regex patterns ────────────────────────────────────────────────────────────

static RESETS_AT_RE: OnceLock<Regex> = OnceLock::new();
static ISO_TIMESTAMP_RE: OnceLock<Regex> = OnceLock::new();
static RETRY_AFTER_SECS_RE: OnceLock<Regex> = OnceLock::new();

/// Matches Claude Code's rate-limit message format:
/// "resets 9pm (Europe/Madrid)" or "resets 10:30am (America/New_York)"
fn resets_at_re() -> &'static Regex {
    RESETS_AT_RE.get_or_init(|| {
        Regex::new(r"resets\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm))\s*\(([^)]+)\)").unwrap()
    })
}

fn iso_timestamp_re() -> &'static Regex {
    ISO_TIMESTAMP_RE
        .get_or_init(|| Regex::new(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)").unwrap())
}

fn retry_after_secs_re() -> &'static Regex {
    RETRY_AFTER_SECS_RE.get_or_init(|| Regex::new(r"(?i)retry[\s_\-]?after[:\s]+(\d+)").unwrap())
}

const DEFAULT_WAIT_SECS: i64 = 300;
const RESUME_BUFFER_SECS: i64 = 15;

/// Detects a rate-limit event from a single raw JSONL line.
/// Returns None if the line is not a rate-limit message.
pub fn detect_rate_limit(jsonl_line: &str) -> Option<RateLimitEvent> {
    let raw = jsonl_line.trim();
    if raw.is_empty() {
        return None;
    }

    let v: Value = serde_json::from_str(raw).ok()?;

    // Primary: structured fields Claude Code sets on rate-limit messages
    let is_rate_limit = v.get("error").and_then(Value::as_str) == Some("rate_limit")
        || v.get("apiErrorStatus").and_then(Value::as_u64) == Some(429)
        || v.get("isApiErrorMessage").and_then(Value::as_bool) == Some(true);

    if !is_rate_limit {
        return None;
    }

    let session_id = extract_session_id(&v)?;
    let reset_at = extract_reset_time(&v, raw);
    let cwd = v.get("cwd").and_then(Value::as_str).map(PathBuf::from);

    debug!(session_id, ?reset_at, "rate limit detected");

    Some(RateLimitEvent {
        session_id,
        reset_at,
        cwd,
    })
}

fn extract_session_id(v: &Value) -> Option<String> {
    v.get("sessionId")
        .or_else(|| v.pointer("/message/sessionId"))
        .and_then(Value::as_str)
        .map(String::from)
}

fn extract_reset_time(v: &Value, raw: &str) -> DateTime<Utc> {
    // 1. Try "resets Xpm (Timezone)" in message content text
    if let Some(text) = extract_message_text(v) {
        if let Some(dt) = parse_resets_at_text(&text) {
            return dt;
        }
    }

    // 2. Try ISO timestamp anywhere in the raw line that's in the future
    if let Some(cap) = iso_timestamp_re().captures(raw) {
        let ts_str = cap.get(1).unwrap().as_str();
        if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
            let parsed: DateTime<Utc> = dt.into();
            if parsed > Utc::now() {
                return parsed + Duration::seconds(RESUME_BUFFER_SECS);
            }
        }
    }

    // 3. Try retry-after in seconds
    if let Some(cap) = retry_after_secs_re().captures(raw) {
        if let Ok(secs) = cap.get(1).unwrap().as_str().parse::<i64>() {
            return Utc::now() + Duration::seconds(secs + RESUME_BUFFER_SECS);
        }
    }

    // 4. Default fallback
    Utc::now() + Duration::seconds(DEFAULT_WAIT_SECS + RESUME_BUFFER_SECS)
}

/// Extracts the text content from Claude Code's synthetic rate-limit message.
fn extract_message_text(v: &Value) -> Option<String> {
    // Structure: message.content[0].text
    v.pointer("/message/content/0/text")
        .and_then(Value::as_str)
        .map(String::from)
}

/// Parses "resets 9pm (Europe/Madrid)" → UTC DateTime
fn parse_resets_at_text(text: &str) -> Option<DateTime<Utc>> {
    let cap = resets_at_re().captures(text)?;
    let time_str = cap.get(1)?.as_str().trim();
    let tz_str = cap.get(2)?.as_str().trim();

    let tz: Tz = tz_str.parse().ok()?;
    let naive_time = parse_ampm_time(time_str)?;

    let now_in_tz = Utc::now().with_timezone(&tz);
    let today = now_in_tz.date_naive();
    let naive_dt = today.and_time(naive_time);

    let reset_local = tz.from_local_datetime(&naive_dt).earliest()?;
    let reset_utc = reset_local.with_timezone(&Utc);

    // If the reset time has already passed today, it must be tomorrow
    let reset_utc = if reset_utc < Utc::now() {
        let tomorrow = today.succ_opt()?;
        let dt = tz
            .from_local_datetime(&tomorrow.and_time(naive_time))
            .earliest()?;
        dt.with_timezone(&Utc)
    } else {
        reset_utc
    };

    Some(reset_utc + Duration::seconds(RESUME_BUFFER_SECS))
}

/// Parses "9pm", "10:30am", "21:00" → NaiveTime
fn parse_ampm_time(s: &str) -> Option<NaiveTime> {
    let s = s.trim().to_lowercase();
    let s = s.replace(' ', "");

    let (time_part, is_pm) = if let Some(t) = s.strip_suffix("pm") {
        (t, true)
    } else if let Some(t) = s.strip_suffix("am") {
        (t, false)
    } else {
        // Try 24h format "21:00"
        return NaiveTime::parse_from_str(&s, "%H:%M").ok();
    };

    let (hour, minute) = if let Some((h, m)) = time_part.split_once(':') {
        (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?)
    } else {
        (time_part.parse::<u32>().ok()?, 0)
    };

    let hour24 = match (is_pm, hour) {
        (true, 12) => 12,
        (true, h) => h + 12,
        (false, 12) => 0,
        (false, h) => h,
    };

    NaiveTime::from_hms_opt(hour24, minute, 0)
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use chrono::Utc;

    #[test]
    fn parse_ampm_time_various() {
        let t = parse_ampm_time("9pm").expect("should parse 9pm");
        assert_eq!(t.hour(), 21);

        let t = parse_ampm_time("10:30am").expect("should parse 10:30am");
        assert_eq!(t.hour(), 10);
        assert_eq!(t.minute(), 30);

        let t = parse_ampm_time("12am").expect("should parse 12am");
        assert_eq!(t.hour(), 0);

        let t = parse_ampm_time("12pm").expect("should parse 12pm");
        assert_eq!(t.hour(), 12);

        let t = parse_ampm_time("21:00").expect("should parse 24h");
        assert_eq!(t.hour(), 21);
    }

    #[test]
    fn parse_resets_at_text_future() {
        // Use a time that is likely later today in UTC to ensure it's parsed
        let text = "You're out of extra usage · resets 11:59pm (UTC)";
        let dt = parse_resets_at_text(text).expect("should parse resets text");
        assert!(dt > Utc::now());
    }
}

