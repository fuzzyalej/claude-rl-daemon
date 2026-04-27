use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

use claude_rl_daemon::DaemonState;

use crate::state;

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Sessions,
    Logs,
}

#[derive(Debug, Clone)]
pub enum Dialog {
    ConfirmCancel { uuid: String },
    ConfirmResume { uuid: String },
    Reschedule { uuid: String, input: String },
    DoctorOutput { lines: Vec<String> },
    LogsFullscreen,
}

pub struct App {
    pub daemon_state: DaemonState,
    pub active_sessions: Vec<(String, PathBuf)>,
    pub session_messages: Vec<String>,
    pub logs: VecDeque<String>,
    pub selected: usize,
    pub session_count: usize,
    pub focus: Focus,
    pub dialog: Option<Dialog>,
    pub last_refresh: Instant,
    pub daemon_running: bool,
    pub error: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            daemon_state: DaemonState::default(),
            active_sessions: Vec::new(),
            session_messages: Vec::new(),
            logs: VecDeque::with_capacity(200),
            selected: 0,
            session_count: 0,
            focus: Focus::Sessions,
            dialog: None,
            last_refresh: Instant::now(),
            daemon_running: false,
            error: None,
        }
    }

    pub fn reload(&mut self) {
        let claude_dir = dirs::home_dir().expect("home dir").join(".claude");
        let active_jsonls = claude_rl_daemon::watcher::discover_active_jsonls(&claude_dir);
        self.active_sessions = active_jsonls
            .into_iter()
            .filter_map(|p| {
                let id = p.file_stem()?.to_str()?.to_string();
                Some((id, p))
            })
            .collect();

        match state::load_state() {
            Ok(s) => {
                // Filter active sessions to exclude those that are already pending
                self.active_sessions.retain(|(id, _)| !s.pending.contains_key(id));

                self.session_count = s.pending.len() + self.active_sessions.len();
                if self.session_count > 0 && self.selected >= self.session_count {
                    self.selected = self.session_count - 1;
                }
                self.daemon_state = s;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("State error: {e}"));
            }
        }
        self.load_logs();
        self.reload_daemon_status();
        self.load_session_messages();
        self.last_refresh = Instant::now();
    }

    pub fn move_down(&mut self) {
        if self.session_count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % self.session_count;
        self.load_session_messages();
    }

    pub fn move_up(&mut self) {
        if self.session_count == 0 {
            return;
        }
        self.selected = self.selected.checked_sub(1).unwrap_or(self.session_count - 1);
        self.load_session_messages();
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sessions => Focus::Logs,
            Focus::Logs => Focus::Sessions,
        };
    }

    pub fn close_dialog(&mut self) {
        self.dialog = None;
    }

    pub fn reload_daemon_status(&mut self) {
        let ok = std::process::Command::new("launchctl")
            .args(["list", "com.claudedaemon"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        self.daemon_running = ok;
    }

    pub fn load_logs_from(&mut self, path: &Path) {
        self.logs.clear();
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().rev().take(200).collect::<Vec<_>>().into_iter().rev() {
                self.logs.push_back(line.to_string());
            }
        }
    }

    pub fn load_logs(&mut self) {
        let path = crate::state::log_path();
        self.load_logs_from(&path);
    }

    pub fn selected_uuid(&self) -> Option<String> {
        if self.selected < self.active_sessions.len() {
            return Some(self.active_sessions[self.selected].0.clone());
        }

        let pending_idx = self.selected - self.active_sessions.len();
        let mut pending: Vec<_> = self.daemon_state.pending.values().collect();
        pending.sort_by_key(|r| r.reset_at);
        pending.get(pending_idx).map(|r| r.session_id.clone())
    }

    pub fn load_session_messages(&mut self) {
        self.session_messages.clear();
        let path = match self.selected_jsonl_path() {
            Some(p) => p,
            None => return,
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().rev().take(50).collect::<Vec<_>>().into_iter().rev() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    let role = val["type"].as_str().or_else(|| val["message"]["role"].as_str());
                    let content = if let Some(c) = val["message"]["content"].as_str() {
                        Some(c.to_string())
                    } else if let Some(arr) = val["message"]["content"].as_array() {
                        let text: Vec<_> = arr.iter()
                            .filter_map(|b| b["text"].as_str())
                            .collect();
                        if text.is_empty() { None } else { Some(text.join("\n")) }
                    } else {
                        None
                    };

                    if let (Some(r), Some(c)) = (role, content) {
                        if r == "user" || r == "assistant" {
                            let label = if r == "user" { "[User]" } else { "[Assistant]" };
                            self.session_messages.push(format!("{} {}", label, c));
                        }
                    }
                }
            }
        }
    }

    fn selected_jsonl_path(&self) -> Option<PathBuf> {
        if self.selected < self.active_sessions.len() {
            return Some(self.active_sessions[self.selected].1.clone());
        }

        let pending_idx = self.selected - self.active_sessions.len();
        let mut pending: Vec<_> = self.daemon_state.pending.values().collect();
        pending.sort_by_key(|r| r.reset_at);
        let r = pending.get(pending_idx)?;

        let cwd = r.cwd.as_ref()?;
        let project_key = claude_rl_daemon::session::cwd_to_project_key(cwd);
        Some(dirs::home_dir()?.join(".claude").join("projects").join(project_key).join(format!("{}.jsonl", r.session_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        App::new()
    }

    #[test]
    fn default_focus_is_sessions() {
        let app = make_app();
        assert_eq!(app.focus, Focus::Sessions);
    }

    #[test]
    fn move_down_wraps_at_end() {
        let mut app = make_app();
        app.selected = 0;
        app.session_count = 3;
        app.move_down();
        assert_eq!(app.selected, 1);
        app.selected = 2;
        app.move_down();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_up_wraps_at_start() {
        let mut app = make_app();
        app.selected = 0;
        app.session_count = 3;
        app.move_up();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = make_app();
        assert_eq!(app.focus, Focus::Sessions);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Logs);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sessions);
    }

    #[test]
    fn set_dialog_and_clear() {
        let mut app = make_app();
        app.dialog = Some(Dialog::ConfirmCancel { uuid: "abc".to_string() });
        assert!(app.dialog.is_some());
        app.close_dialog();
        assert!(app.dialog.is_none());
    }

    #[test]
    fn load_logs_fills_deque() {
        let mut app = make_app();
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "line one").unwrap();
        writeln!(f, "line two").unwrap();
        app.load_logs_from(f.path());
        assert_eq!(app.logs.len(), 2);
        assert!(app.logs.back().unwrap().contains("line two"));
    }

    #[test]
    fn selected_uuid_returns_active_or_pending() {
        let mut app = make_app();
        app.active_sessions = vec![
            ("active1".to_string(), PathBuf::from("p1")),
            ("active2".to_string(), PathBuf::from("p2")),
        ];
        app.daemon_state.pending.insert("pending1".to_string(), claude_rl_daemon::PendingResume {
            session_id: "pending1".to_string(),
            reset_at: chrono::Utc::now(),
            cwd: None,
        });
        app.session_count = 3;

        assert_eq!(app.selected_uuid(), Some("active1".to_string()));
        app.selected = 1;
        assert_eq!(app.selected_uuid(), Some("active2".to_string()));
        app.selected = 2;
        assert_eq!(app.selected_uuid(), Some("pending1".to_string()));
        app.selected = 3;
        assert_eq!(app.selected_uuid(), None);
    }
}
