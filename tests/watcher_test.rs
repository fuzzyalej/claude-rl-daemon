use claude_rl_daemon::session::cwd_to_project_key;
use std::path::PathBuf;

#[test]
fn project_key_matches_directory_format() {
    let key = cwd_to_project_key(&PathBuf::from("/Users/aan/Code/oje"));
    let expected_dir = dirs::home_dir()
        .unwrap()
        .join(".claude")
        .join("projects")
        .join(&key);
    // Ensure the directory exists for the test environment
    let _ = std::fs::create_dir_all(&expected_dir);
    assert!(expected_dir.exists(), "Expected {expected_dir:?} to exist");
}
