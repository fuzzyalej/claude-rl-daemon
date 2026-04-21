use claude_rl_daemon::detector::detect_rate_limit;

// Real Claude Code rate-limit message format (observed 2026-04-20)
const REAL_RATE_LIMIT_LINE: &str = r#"{"parentUuid":"986e9b92-e21e-476d-8808-b43912d106a5","isSidechain":false,"type":"assistant","uuid":"f3c95bf7-6d74-4dad-976c-a81065a9970b","timestamp":"2026-04-20T17:27:18.779Z","message":{"id":"e5bbbf08-cfab-4571-969f-6afa8a3c6b8d","model":"<synthetic>","role":"assistant","stop_reason":"stop_sequence","type":"message","content":[{"type":"text","text":"You're out of extra usage \u00b7 resets 9pm (Europe/Madrid)"}]},"error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429,"userType":"external","entrypoint":"cli","cwd":"/Users/aan/Code/oje","sessionId":"fc456884-d0b4-45f8-9d53-9a64dbc663d6","version":"2.1.114","gitBranch":"main"}"#;

#[test]
fn detects_real_rate_limit_message() {
    let event = detect_rate_limit(REAL_RATE_LIMIT_LINE);
    assert!(event.is_some(), "should detect real rate-limit message");
    let e = event.unwrap();
    assert_eq!(e.session_id, "fc456884-d0b4-45f8-9d53-9a64dbc663d6");
    assert_eq!(e.cwd.unwrap().to_str().unwrap(), "/Users/aan/Code/oje");
    // reset_at should be in the future (either today or tomorrow at 9pm Madrid + 15s buffer)
    assert!(
        e.reset_at > chrono::Utc::now(),
        "reset_at should be in the future"
    );
}

#[test]
fn extracts_session_id_from_real_message() {
    let e = detect_rate_limit(REAL_RATE_LIMIT_LINE).unwrap();
    assert_eq!(e.session_id, "fc456884-d0b4-45f8-9d53-9a64dbc663d6");
}

#[test]
fn detects_via_error_field() {
    let line = r#"{"type":"assistant","error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429,"sessionId":"abc-123","cwd":"/tmp","message":{"content":[{"type":"text","text":"quota exhausted"}]}}"#;
    let event = detect_rate_limit(line);
    assert!(event.is_some());
    assert_eq!(event.unwrap().session_id, "abc-123");
}

#[test]
fn detects_via_api_error_status_429() {
    let line = r#"{"type":"assistant","error":"rate_limit","apiErrorStatus":429,"sessionId":"abc-456","cwd":"/tmp","message":{"content":[{"type":"text","text":"too many requests"}]}}"#;
    assert!(detect_rate_limit(line).is_some());
}

#[test]
fn ignores_normal_assistant_message() {
    let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello, how can I help?"}]},"sessionId":"abc-789","cwd":"/tmp"}"#;
    assert!(detect_rate_limit(line).is_none());
}

#[test]
fn ignores_normal_user_message() {
    let line = r#"{"type":"user","message":{"role":"user","content":"hello"},"sessionId":"abc-789","cwd":"/tmp"}"#;
    assert!(detect_rate_limit(line).is_none());
}

#[test]
fn parses_resets_at_with_future_time() {
    // Construct a line with a time far in the future to guarantee it's "today"
    let line = r#"{"type":"assistant","error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429,"sessionId":"s1","cwd":"/tmp","message":{"content":[{"type":"text","text":"resets 11:59pm (UTC)"}]}}"#;
    let event = detect_rate_limit(line).unwrap();
    assert!(
        event.reset_at > chrono::Utc::now(),
        "reset_at should be in the future"
    );
    // Should include the 15s buffer on top of 11:59pm UTC
    let secs_from_now = event.reset_at.timestamp() - chrono::Utc::now().timestamp();
    assert!(secs_from_now > 0, "should be future: {secs_from_now}s");
}

#[test]
fn falls_back_to_default_wait_when_no_reset_time() {
    // Message has rate_limit fields but no parseable reset time in content
    let line = r#"{"type":"assistant","error":"rate_limit","apiErrorStatus":429,"sessionId":"s2","cwd":"/tmp","message":{"content":[{"type":"text","text":"Usage limit reached."}]}}"#;
    let event = detect_rate_limit(line).unwrap();
    let diff = event.reset_at.timestamp() - chrono::Utc::now().timestamp();
    assert!(
        diff > 200 && diff < 400,
        "Expected ~5 min default, got {diff}s"
    );
}

#[test]
fn parses_iso_timestamp_in_raw_line() {
    // Include an ISO timestamp in the raw line well in the future
    let future = (chrono::Utc::now() + chrono::Duration::seconds(3600)).to_rfc3339();
    let line = format!(r#"{{"error":"rate_limit","sessionId":"iso-1","message":{{"content":[{{"type":"text","text":"ok"}}]}},"timestamp":"{}"}}"#, future);
    let event = detect_rate_limit(&line).unwrap();
    assert_eq!(event.session_id, "iso-1");
    assert!(event.reset_at > chrono::Utc::now());
}

#[test]
fn parses_retry_after_seconds_in_raw_line() {
    // Include 'retry after 10' in the raw text
    let line = r#"{"error":"rate_limit","sessionId":"retry-1","message":{"content":[{"type":"text","text":"please retry after 10"}]}}"#;
    let event = detect_rate_limit(line).unwrap();
    let diff = event.reset_at.timestamp() - chrono::Utc::now().timestamp();
    assert!(diff >= 10);
}
