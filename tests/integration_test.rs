use claude_rl_daemon::detector::detect_rate_limit;
use claude_rl_daemon::scheduler::Scheduler;
use claude_rl_daemon::tmux::tmux_session_name;
use tempfile::tempdir;

#[tokio::test]
async fn full_pipeline_deduplicates() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));

    let line = r#"{"type":"system","content":"rate limit exceeded, please wait","sessionId":"full-test-session","timestamp":"2026-04-20T14:30:00Z","uuid":"u1","cwd":"/tmp"}"#;

    let event = detect_rate_limit(line).unwrap();
    assert!(sched.try_schedule(event.clone()).await.unwrap());

    // Second call for same session → rejected
    let event2 = detect_rate_limit(line).unwrap();
    assert!(!sched.try_schedule(event2).await.unwrap());

    assert_eq!(tmux_session_name("full-test-session"), "claude-rl-full-tes");
}
