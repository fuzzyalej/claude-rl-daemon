use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use super::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    draw_status_bar(frame, app, chunks[0]);
    draw_sessions(frame, app, chunks[1]);
    draw_messages(frame, app, chunks[2]);
    draw_logs(frame, app, chunks[3]);
    draw_keybinds(frame, chunks[4]);

    if app.dialog.is_some() {
        super::dialogs::draw_dialog(frame, app);
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let elapsed = app.last_refresh.elapsed().as_secs();
    let daemon_label = if app.daemon_running {
        Span::styled("● running", Style::default().fg(Color::Green))
    } else {
        Span::styled("○ stopped", Style::default().fg(Color::Red))
    };
    
    let mut spans = vec![
        daemon_label,
        Span::raw(format!("  Updated: {}s ago  [r] refresh  [q] quit", elapsed)),
    ];

    if let Some(err) = &app.error {
        spans.push(Span::styled(format!("  ERROR: {}", err), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_sessions(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use crate::format;
    use claude_rl_daemon::PendingResume;

    let block = Block::default()
        .title(format!(
            " SESSIONS ({} active, {} pending, {} resumed) ",
            app.active_sessions.len(),
            app.daemon_state.pending.len(),
            app.daemon_state.completed.len()
        ))
        .borders(Borders::ALL);

    let mut rows: Vec<Row> = Vec::new();

    // 1. Active sessions
    for (i, (session_id, _path)) in app.active_sessions.iter().enumerate() {
        let cursor = if i == app.selected { "▶" } else { " " };
        let uuid_short = &session_id[..8.min(session_id.len())];

        let style = if i == app.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Green)
        };

        rows.push(
            Row::new(vec![
                cursor.to_string(),
                "●".to_string(),
                uuid_short.to_string(),
                "—".to_string(),
                "—".to_string(),
                "active".to_string(),
            ])
            .style(style),
        );
    }

    // 2. Pending sessions
    let pending: Vec<PendingResume> = app.daemon_state.pending.values().cloned().collect();
    let sorted = format::sorted_pending(&pending);

    for (i, r) in sorted.iter().enumerate() {
        let idx = i + app.active_sessions.len();
        let cursor = if idx == app.selected { "▶" } else { " " };
        let uuid_short = &r.session_id[..8.min(r.session_id.len())];
        let cwd = r
            .cwd
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "—".into());
        let reset = format::format_reset_at(r.reset_at);

        let style = if idx == app.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        rows.push(
            Row::new(vec![
                cursor.to_string(),
                format!("{}", i + 1),
                uuid_short.to_string(),
                cwd,
                reset,
                "pending".to_string(),
            ])
            .style(style),
        );
    }

    // 3. Completed sessions
    let mut completed: Vec<_> = app.daemon_state.completed.iter().collect();
    completed.sort();
    for (i, id) in completed.iter().enumerate() {
        let idx = i + app.active_sessions.len() + app.daemon_state.pending.len();
        let cursor = if idx == app.selected { "▶" } else { " " };
        let uuid_short = &id[..8.min(id.len())];

        let style = if idx == app.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        rows.push(
            Row::new(vec![
                cursor.to_string(),
                format!("{}", i + 1 + app.daemon_state.pending.len()),
                uuid_short.to_string(),
                "—".to_string(),
                "—".to_string(),
                "resumed".to_string(),
            ])
            .style(style),
        );
    }

    let widths = [
        Constraint::Length(2),
        Constraint::Length(4),
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(16),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "#", "UUID", "Project", "Reset At", "Status"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(block);

    frame.render_widget(table, area);
}

fn draw_messages(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" SESSION MESSAGES ").borders(Borders::ALL);
    let lines: Vec<Line> = app
        .session_messages
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|l| Line::from(l.as_str()))
        .collect();
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_logs(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" LOGS ").borders(Borders::ALL);
    let lines: Vec<Line> = app.logs.iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|l| Line::from(l.as_str()))
        .collect();
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_keybinds(frame: &mut Frame, area: ratatui::layout::Rect) {
    let hints = "[↑↓] select  [h] hook  [x] cancel  [e] resume  [s] reschedule  [d] doctor  [l] logs  [Tab] focus";
    frame.render_widget(Paragraph::new(hints), area);
}
