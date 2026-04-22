use chrono::{Duration, Utc};
use claude_rl_daemon::detector::RateLimitEvent;
use claude_rl_daemon::scheduler::Scheduler;
use std::path::PathBuf;
use tempfile::tempdir;

fn make_event(session_id: &str, secs_from_now: i64) -> RateLimitEvent {
    RateLimitEvent {
        session_id: session_id.to_string(),
        reset_at: Utc::now() + Duration::seconds(secs_from_now),
        cwd: Some(PathBuf::from("/tmp")),
    }
}

#[tokio::test]
async fn schedules_new_event() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));
    let event = make_event("session-1", 600);
    let scheduled = sched.try_schedule(event).await.unwrap();
    assert!(scheduled);
}

#[tokio::test]
async fn deduplicates_same_session() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));
    let e1 = make_event("session-2", 600);
    let e2 = make_event("session-2", 600);
    assert!(sched.try_schedule(e1).await.unwrap());
    assert!(!sched.try_schedule(e2).await.unwrap());
}

#[tokio::test]
async fn persists_and_reloads_state() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("state.json");

    {
        let mut sched = Scheduler::new(state_path.clone());
        sched
            .try_schedule(make_event("session-3", 600))
            .await
            .unwrap();
    }

    let sched2 = Scheduler::new(state_path);
    assert!(sched2.is_pending("session-3"));
}

#[test]
fn completed_sessions_not_pending() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));
    sched.mark_completed("session-4");
    assert!(!sched.is_pending("session-4"));
}

#[tokio::test]
async fn all_pending_returns_scheduled_events() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));
    assert!(sched.all_pending().is_empty());

    sched.try_schedule(make_event("session-5", 600)).await.unwrap();
    sched.try_schedule(make_event("session-6", 600)).await.unwrap();

    let pending = sched.all_pending();
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn completed_session_not_rescheduled() {
    let dir = tempdir().unwrap();
    let mut sched = Scheduler::new(dir.path().join("state.json"));
    sched.try_schedule(make_event("session-7", 600)).await.unwrap();
    sched.mark_completed("session-7");
    // Completed sessions must not be re-scheduled
    let result = sched.try_schedule(make_event("session-7", 600)).await.unwrap();
    assert!(!result);
}
