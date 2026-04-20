use claude_rl_daemon::detector::detect_rate_limit;

#[test]
fn detects_429_in_assistant_message() {
    let line = r#"{"type":"assistant","message":{"content":"API error 429: Too Many Requests. Retry after 2026-04-20T15:30:00Z"},"sessionId":"abc-123","timestamp":"2026-04-20T14:30:00Z","uuid":"u1","cwd":"/tmp"}"#;
    let event = detect_rate_limit(line);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.session_id, "abc-123");
    assert!(e.reset_at > chrono::Utc::now());
}

#[test]
fn detects_rate_limit_keyword() {
    let line = r#"{"type":"system","subtype":"api_error","content":"rate limit exceeded, please wait","sessionId":"abc-456","timestamp":"2026-04-20T14:30:00Z","uuid":"u2","cwd":"/tmp"}"#;
    let event = detect_rate_limit(line);
    assert!(event.is_some());
    assert_eq!(event.unwrap().session_id, "abc-456");
}

#[test]
fn ignores_normal_message() {
    let line = r#"{"type":"user","message":{"role":"user","content":"hello"},"sessionId":"abc-789","timestamp":"2026-04-20T14:00:00Z","uuid":"u3","cwd":"/tmp"}"#;
    assert!(detect_rate_limit(line).is_none());
}

#[test]
fn extracts_iso_reset_time() {
    let line = r#"{"type":"system","content":"retry after 2099-04-20T16:00:00Z","sessionId":"s1","timestamp":"2026-04-20T14:00:00Z","uuid":"u4","cwd":"/tmp"}"#;
    let event = detect_rate_limit(line).unwrap();
    let expected = chrono::DateTime::parse_from_rfc3339("2099-04-20T16:00:00Z").unwrap();
    assert_eq!(event.reset_at.timestamp(), expected.timestamp() + 15);
}

#[test]
fn falls_back_to_default_wait_when_no_timestamp() {
    let line = r#"{"type":"system","content":"quota exhausted","sessionId":"s2","timestamp":"2026-04-20T14:00:00Z","uuid":"u5","cwd":"/tmp"}"#;
    let event = detect_rate_limit(line).unwrap();
    let diff = event.reset_at.timestamp() - chrono::Utc::now().timestamp();
    assert!(diff > 200 && diff < 400, "Expected ~5 min default, got {diff}s");
}
