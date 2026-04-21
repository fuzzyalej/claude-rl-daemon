use claude_rl_daemon::session::{cwd_to_project_key, jsonl_path, SessionEntry, SessionMessage};
use std::path::PathBuf;

#[test]
fn parses_session_entry() {
    let json = r#"{"pid":30531,"sessionId":"fc456884-d0b4-45f8-9d53-9a64dbc663d6","cwd":"/Users/aan/Code/oje","startedAt":1776697513701,"version":"2.1.114","kind":"interactive","entrypoint":"cli"}"#;
    let entry: SessionEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.session_id, "fc456884-d0b4-45f8-9d53-9a64dbc663d6");
    assert_eq!(entry.pid, 30531);
    assert_eq!(entry.cwd.to_str().unwrap(), "/Users/aan/Code/oje");
}

#[test]
fn parses_last_prompt_message() {
    let json = r#"{"type":"last-prompt","lastPrompt":"some prompt","sessionId":"abc-123"}"#;
    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, SessionMessage::LastPrompt { .. }));
}

#[test]
fn parses_system_message() {
    let json = r#"{"type":"system","subtype":"turn_duration","durationMs":167177,"sessionId":"abc-123","timestamp":"2026-04-19T08:17:53.678Z"}"#;
    let msg: SessionMessage = serde_json::from_str(json).unwrap();
    assert!(matches!(msg, SessionMessage::System { .. }));
}

#[test]
fn cwd_to_project_key_converts_correctly() {
    let cwd = PathBuf::from("/Users/aan/Code/oje");
    assert_eq!(cwd_to_project_key(&cwd), "-Users-aan-Code-oje");
}

#[test]
fn jsonl_path_contains_session_id_and_ext() {
    let entry = SessionEntry {
        pid: 1,
        session_id: "session-xyz".to_string(),
        cwd: PathBuf::from("/Users/aan/Code/oje"),
        started_at: 0,
        version: "v".to_string(),
        kind: "k".to_string(),
        entrypoint: "e".to_string(),
    };

    let p = jsonl_path(&entry);
    assert!(p.extension().is_some() && p.extension().unwrap() == "jsonl");
    assert!(p.to_string_lossy().contains(&entry.session_id));
}
