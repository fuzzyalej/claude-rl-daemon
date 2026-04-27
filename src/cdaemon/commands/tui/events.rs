use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

use super::app::{App, Dialog};
use crate::commands::{cancel, resume};
use crate::state;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub fn next_event(tick_ms: u64) -> anyhow::Result<Option<AppEvent>> {
    if event::poll(Duration::from_millis(tick_ms))? {
        match event::read()? {
            Event::Key(k) => Ok(Some(AppEvent::Key(k))),
            _ => Ok(None),
        }
    } else {
        Ok(Some(AppEvent::Tick))
    }
}

/// Returns true if the app should quit.
pub fn handle_key(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    if app.dialog.is_some() {
        return handle_dialog_key(app, key, terminal);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('r') => app.reload(),
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Down => app.move_down(),
        KeyCode::Up => app.move_up(),
        KeyCode::Char('x') => {
            if let Some(uuid) = app.selected_uuid() {
                app.dialog = Some(Dialog::ConfirmCancel { uuid });
            }
        }
        KeyCode::Char('e') => {
            if let Some(uuid) = app.selected_uuid() {
                app.dialog = Some(Dialog::ConfirmResume { uuid });
            }
        }
        KeyCode::Char('s') => {
            if let Some(uuid) = app.selected_uuid() {
                app.dialog = Some(Dialog::Reschedule { uuid, input: String::new() });
            }
        }
        KeyCode::Char('l') => {
            app.dialog = Some(Dialog::LogsFullscreen);
        }
        KeyCode::Char('d') => {
            app.dialog = Some(Dialog::DoctorOutput { lines: run_doctor() });
        }
        KeyCode::Char('h') => {
            if let Some(uuid) = app.selected_uuid() {
                if app.selected < app.active_sessions.len() {
                    app.error = Some("Session is active in your terminal, not managed by daemon tmux.".to_string());
                } else {
                    attach_tmux(app, terminal, &uuid)?;
                }
            }
        }
        _ => {}
    }

    Ok(false)
}

fn handle_dialog_key(
    app: &mut App,
    key: KeyEvent,
    _terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<bool> {
    match &app.dialog.clone() {
        Some(Dialog::ConfirmCancel { uuid }) => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let mut state = state::load_state()?;
                    let _ = cancel::execute(&mut state, uuid);
                    state::save_state(&state)?;
                    app.close_dialog();
                    app.reload();
                }
                _ => app.close_dialog(),
            }
        }
        Some(Dialog::ConfirmResume { uuid }) => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    // TODO: replace with resume::execute once Task 5 is done
                    let _ = resume::run(uuid);
                    app.close_dialog();
                    app.reload();
                }
                _ => app.close_dialog(),
            }
        }
        Some(Dialog::Reschedule { uuid, input }) => {
            let uuid = uuid.clone();
            let input = input.clone();
            match key.code {
                KeyCode::Esc => app.close_dialog(),
                KeyCode::Enter => {
                    if !input.is_empty() {
                        if let Ok(mut state) = state::load_state() {
                            let _ = crate::commands::reschedule::execute(&mut state, &uuid, &input);
                            let _ = state::save_state(&state);
                        }
                    }
                    app.close_dialog();
                    app.reload();
                }
                KeyCode::Backspace => {
                    if let Some(Dialog::Reschedule { input, .. }) = &mut app.dialog {
                        input.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(Dialog::Reschedule { input, .. }) = &mut app.dialog {
                        input.push(c);
                    }
                }
                _ => {}
            }
        }
        Some(Dialog::DoctorOutput { .. }) | Some(Dialog::LogsFullscreen) => {
            if key.code == KeyCode::Esc {
                app.close_dialog();
            }
        }
        None => {}
    }

    Ok(false)
}

fn run_doctor() -> Vec<String> {
    let out = std::process::Command::new(std::env::current_exe().unwrap_or_default())
        .arg("doctor")
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect(),
        Err(e) => vec![format!("Error running doctor: {e}")],
    }
}

fn attach_tmux(
    app: &mut App,
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    uuid: &str,
) -> anyhow::Result<()> {
    use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
    use claude_rl_daemon::tmux::{find_tmux_binary, tmux_session_name};

    let tmux_name = tmux_session_name(uuid);
    let tmux_bin = find_tmux_binary();

    // Check if session exists first
    let check = std::process::Command::new(&tmux_bin)
        .args(["has-session", "-t", &tmux_name])
        .output();

    match check {
        Ok(out) if !out.status.success() => {
            app.error = Some(format!("tmux session '{tmux_name}' not found. It may have exited or not been resumed yet."));
            return Ok(());
        }
        Err(_) => {
            app.error = Some("tmux binary not found. Please install tmux.".to_string());
            return Ok(());
        }
        _ => {}
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    let status = std::process::Command::new(&tmux_bin)
        .args(["attach", "-t", &tmux_name])
        .status();

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;

    if let Ok(s) = status {
        if !s.success() {
            app.error = Some(format!("tmux attach failed with status {s}"));
        }
    } else if let Err(e) = status {
        app.error = Some(format!("Failed to run tmux attach: {e}"));
    }

    app.reload();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::app::{App, Dialog, Focus};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn app_with_sessions(n: usize) -> App {
        let mut app = App::new();
        app.session_count = n;
        app
    }

    #[test]
    fn x_opens_cancel_dialog_when_session_exists() {
        let mut app = app_with_sessions(1);
        app.dialog = Some(Dialog::ConfirmCancel { uuid: "test-uuid".to_string() });
        assert!(matches!(app.dialog, Some(Dialog::ConfirmCancel { .. })));
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Sessions);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Logs);
    }

    #[test]
    fn esc_closes_dialog() {
        let mut app = App::new();
        app.dialog = Some(Dialog::LogsFullscreen);
        app.close_dialog();
        assert!(app.dialog.is_none());
    }
}
