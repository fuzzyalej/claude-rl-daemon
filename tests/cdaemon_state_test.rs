use claude_rl_daemon::scheduler::{DaemonState, PendingResume};
use chrono::Utc;
use tempfile::NamedTempFile;

#[test]
fn loads_state_from_json_file() {
    let state = DaemonState::default();
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), serde_json::to_vec(&state).unwrap()).unwrap();

    let loaded = DaemonState::load_from_path(file.path()).unwrap();
    assert_eq!(loaded.pending.len(), 0);
    assert_eq!(loaded.completed.len(), 0);
}

#[test]
fn load_returns_default_when_file_missing() {
    let state = DaemonState::load_from_path(
        std::path::Path::new("/nonexistent/state.json")
    ).unwrap();
    assert_eq!(state.pending.len(), 0);
}

#[test]
fn roundtrip_with_pending_session() {
    let mut state = DaemonState::default();
    state.pending.insert("abc-123".to_string(), PendingResume {
        session_id: "abc-123".to_string(),
        reset_at: Utc::now(),
        cwd: Some(std::path::PathBuf::from("/tmp")),
    });

    let file = NamedTempFile::new().unwrap();
    state.save_to_path(file.path()).unwrap();

    let loaded = DaemonState::load_from_path(file.path()).unwrap();
    assert!(loaded.pending.contains_key("abc-123"));
}
