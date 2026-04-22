use claude_rl_daemon::tmux::{build_tmux_args, tmux_session_name};
use std::path::PathBuf;

#[test]
fn session_name_uses_uuid_prefix() {
    let name = tmux_session_name("fc456884-d0b4-45f8-9d53-9a64dbc663d6");
    assert_eq!(name, "claude-rl-fc456884");
}

#[test]
fn session_name_short_id_not_truncated() {
    let name = tmux_session_name("abc");
    assert_eq!(name, "claude-rl-abc");
}

#[test]
fn session_name_exactly_8_chars() {
    let name = tmux_session_name("12345678");
    assert_eq!(name, "claude-rl-12345678");
}

#[test]
fn builds_correct_tmux_args() {
    let args = build_tmux_args(
        "claude-rl-fc456884",
        &PathBuf::from("/Users/aan/Code/oje"),
        "fc456884-d0b4-45f8-9d53-9a64dbc663d6",
    );
    assert_eq!(args[0], "new-session");
    assert!(args.contains(&"-d".to_string()));
    assert!(args.contains(&"-s".to_string()));
    assert!(args.contains(&"claude-rl-fc456884".to_string()));
    let cmd = args.last().unwrap();
    assert!(cmd.contains("--resume"));
    assert!(cmd.contains("fc456884-d0b4-45f8-9d53-9a64dbc663d6"));
}

#[test]
fn build_tmux_args_includes_cwd() {
    let cwd = PathBuf::from("/home/user/project");
    let args = build_tmux_args("some-name", &cwd, "session-123");
    assert!(args.contains(&"-c".to_string()));
    assert!(args.contains(&"/home/user/project".to_string()));
}

#[test]
fn build_tmux_args_has_7_elements() {
    let args = build_tmux_args("name", &PathBuf::from("/tmp"), "id");
    assert_eq!(args.len(), 7);
}
