use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde_json::Value;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct RateLimitEvent {
    pub session_id: String,
    pub reset_at: DateTime<Utc>,
    pub cwd: Option<PathBuf>,
}

// ── Pattern constants ────────────────────────────────────────────────────────
// UPDATE after running scripts/phase1-after.sh with real rate-limit data.

static RATE_LIMIT_RE: OnceLock<Regex> = OnceLock::new();
static ISO_TIMESTAMP_RE: OnceLock<Regex> = OnceLock::new();
static RETRY_AFTER_SECS_RE: OnceLock<Regex> = OnceLock::new();

fn rate_limit_re() -> &'static Regex {
    RATE_LIMIT_RE.get_or_init(|| {
        Regex::new(
            r"(?i)(rate[\s_\-]?limit|too[\s_\-]many[\s_\-]requests|quota[\s_\-](?:exceeded|exhausted)|usage[\s_\-]?limit|overloaded|retry[\s_\-]after|429)",
        )
        .unwrap()
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

pub fn detect_rate_limit(jsonl_line: &str) -> Option<RateLimitEvent> {
    let raw = jsonl_line.trim();
    if raw.is_empty() {
        return None;
    }
    if !rate_limit_re().is_match(raw) {
        return None;
    }

    let v: Value = serde_json::from_str(raw).ok()?;
    let session_id = extract_session_id(&v)?;
    let reset_at = extract_reset_time(raw);
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

fn extract_reset_time(raw: &str) -> DateTime<Utc> {
    if let Some(cap) = iso_timestamp_re().captures(raw) {
        let ts_str = cap.get(1).unwrap().as_str();
        if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
            let parsed: DateTime<Utc> = dt.into();
            if parsed > Utc::now() {
                return parsed + Duration::seconds(RESUME_BUFFER_SECS);
            }
        }
    }

    if let Some(cap) = retry_after_secs_re().captures(raw) {
        if let Ok(secs) = cap.get(1).unwrap().as_str().parse::<i64>() {
            return Utc::now() + Duration::seconds(secs + RESUME_BUFFER_SECS);
        }
    }

    Utc::now() + Duration::seconds(DEFAULT_WAIT_SECS + RESUME_BUFFER_SECS)
}
