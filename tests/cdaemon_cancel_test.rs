use claude_rl_daemon::{DaemonState, PendingResume};
use chrono::Utc;
use tempfile::NamedTempFile;

fn make_state_with_session(id: &str) -> (DaemonState, NamedTempFile) {
    let mut state = DaemonState::default();
    state.pending.insert(id.to_string(), PendingResume {
        session_id: id.to_string(),
        reset_at: Utc::now(),
        cwd: None,
    });
    let file = NamedTempFile::new().unwrap();
    state.save_to_path(file.path()).unwrap();
    (state, file)
}

#[test]
fn cancel_removes_session_from_state() {
    let id = "fc456884-d0b4-45f8-9d53-9a64dbc663d6";
    let (_state, file) = make_state_with_session(id);

    let mut loaded = DaemonState::load_from_path(file.path()).unwrap();
    assert!(loaded.pending.contains_key(id));

    loaded.pending.remove(id);
    loaded.save_to_path(file.path()).unwrap();

    let reloaded = DaemonState::load_from_path(file.path()).unwrap();
    assert!(!reloaded.pending.contains_key(id));
}

#[test]
fn prefix_match_finds_session() {
    let id = "fc456884-d0b4-45f8-9d53-9a64dbc663d6";
    let (state, _file) = make_state_with_session(id);

    let found = state.pending.keys().find(|k| k.starts_with("fc456884")).cloned();
    assert_eq!(found.unwrap(), id);
}
