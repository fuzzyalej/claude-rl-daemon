use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use claude_rl_daemon::detector::detect_rate_limit;
use claude_rl_daemon::scheduler::Scheduler;
use claude_rl_daemon::tmux::tmux_session_name;
use claude_rl_daemon::watcher::{handle_change, read_new_lines};
use tempfile::tempdir;
use tokio::sync::{mpsc, Mutex};

const RATE_LIMIT_LINE: &str = r#"{"type":"assistant","error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429,"sessionId":"full-test-session","cwd":"/tmp","message":{"content":[{"type":"text","text":"You're out of extra usage · resets 11:59pm (UTC)"}]}}"#;

#[tokio::test]
async fn full_pipeline_deduplicates() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));

    let event = detect_rate_limit(RATE_LIMIT_LINE).unwrap();
    assert!(sched.try_schedule(event.clone()).await.unwrap());

    let event2 = detect_rate_limit(RATE_LIMIT_LINE).unwrap();
    assert!(!sched.try_schedule(event2).await.unwrap());

    assert_eq!(tmux_session_name("full-test-session"), "claude-rl-full-tes");
}

#[tokio::test]
async fn watcher_detects_rate_limit_in_jsonl_and_schedules() {
    let dir = tempdir().unwrap();
    let claude_dir = dir.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();

    let jsonl = dir.path().join("session.jsonl");
    std::fs::write(&jsonl, format!("{RATE_LIMIT_LINE}\n")).unwrap();

    let scheduler = Arc::new(Mutex::new(Scheduler::new(dir.path().join("state.json"))));
    let offsets = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _rx) = mpsc::channel::<PathBuf>(16);

    handle_change(jsonl, &claude_dir, Arc::clone(&scheduler), offsets, tx).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(scheduler.lock().await.is_pending("full-test-session"));
}

#[tokio::test]
async fn watcher_does_not_double_schedule_same_session() {
    let dir = tempdir().unwrap();
    let claude_dir = dir.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();

    let jsonl = dir.path().join("session.jsonl");
    std::fs::write(&jsonl, format!("{RATE_LIMIT_LINE}\n")).unwrap();

    let scheduler = Arc::new(Mutex::new(Scheduler::new(dir.path().join("state.json"))));
    let offsets = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _rx) = mpsc::channel::<PathBuf>(16);

    handle_change(jsonl.clone(), &claude_dir, Arc::clone(&scheduler), Arc::clone(&offsets), tx.clone()).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(scheduler.lock().await.is_pending("full-test-session"));

    // Append the same line again — should not create a second entry
    let mut file = std::fs::OpenOptions::new().append(true).open(&jsonl).unwrap();
    writeln!(file, "{RATE_LIMIT_LINE}").unwrap();
    drop(file);

    handle_change(jsonl, &claude_dir, Arc::clone(&scheduler), Arc::clone(&offsets), tx).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(scheduler.lock().await.all_pending().len(), 1);
}

#[tokio::test]
async fn incremental_read_only_delivers_new_lines() {
    let dir = tempdir().unwrap();
    let mut f = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    writeln!(f, "first").unwrap();

    let offsets = Arc::new(Mutex::new(HashMap::new()));
    let path = f.path().to_path_buf();

    let first = read_new_lines(&path, Arc::clone(&offsets)).await;
    assert_eq!(first, vec!["first"]);

    writeln!(f, "second").unwrap();
    let second = read_new_lines(&path, Arc::clone(&offsets)).await;
    assert_eq!(second, vec!["second"]);
}

#[tokio::test]
async fn scheduler_survives_restart_with_pending_resumes() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("state.json");

    {
        let mut sched = Scheduler::new(state_path.clone());
        let event = detect_rate_limit(RATE_LIMIT_LINE).unwrap();
        sched.try_schedule(event).await.unwrap();
    }

    // Simulate daemon restart — new Scheduler loads from disk
    let sched2 = Scheduler::new(state_path);
    assert!(sched2.is_pending("full-test-session"));
    assert_eq!(sched2.all_pending().len(), 1);
}
