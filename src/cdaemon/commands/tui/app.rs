use std::collections::VecDeque;
use std::path::Path;
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
        match state::load_state() {
            Ok(s) => {
                self.session_count = s.pending.len();
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
        self.last_refresh = Instant::now();
    }

    pub fn move_down(&mut self) {
        if self.session_count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % self.session_count;
    }

    pub fn move_up(&mut self) {
        if self.session_count == 0 {
            return;
        }
        self.selected = self.selected.checked_sub(1).unwrap_or(self.session_count - 1);
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
        let mut pending: Vec<_> = self.daemon_state.pending.values().collect();
        pending.sort_by_key(|r| r.reset_at);
        pending.get(self.selected).map(|r| r.session_id.clone())
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
}
