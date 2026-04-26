use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::app::{App, Dialog};

pub fn draw_dialog(frame: &mut Frame, app: &App) {
    match &app.dialog {
        Some(Dialog::ConfirmCancel { uuid }) => draw_confirm(
            frame,
            &format!("Cancel resume for {}?", &uuid[..8.min(uuid.len())]),
            "[y] Yes  [any] No",
        ),
        Some(Dialog::ConfirmResume { uuid }) => draw_confirm(
            frame,
            &format!("Resume {} now?", &uuid[..8.min(uuid.len())]),
            "[y] Yes  [any] No",
        ),
        Some(Dialog::Reschedule { uuid, input }) => draw_input(
            frame,
            &format!("Reschedule {} — new time:", &uuid[..8.min(uuid.len())]),
            input,
        ),
        Some(Dialog::DoctorOutput { lines }) => draw_scrollable(frame, " Doctor ", lines),
        Some(Dialog::LogsFullscreen) => draw_scrollable(
            frame,
            " Logs ",
            &app.logs.iter().cloned().collect::<Vec<_>>(),
        ),
        None => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_confirm(frame: &mut Frame, title: &str, hint: &str) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default().title(title).borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), inner);
}

fn draw_input(frame: &mut Frame, title: &str, input: &str) {
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default().title(title).borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let content = Line::from(vec![
        Span::raw(input),
        Span::styled("█", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]);
    frame.render_widget(Paragraph::new(content), inner);
}

fn draw_scrollable(frame: &mut Frame, title: &str, lines: &[String]) {
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default().title(format!("{title} [Esc] close")).borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text: Vec<Line> = lines.iter().map(|l| Line::from(l.as_str())).collect();
    frame.render_widget(Paragraph::new(text), inner);
}
