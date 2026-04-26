mod app;
mod dialogs;
mod events;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{next_event, AppEvent};
use ratatui::{backend::CrosstermBackend, Terminal};

const REFRESH_INTERVAL_SECS: u64 = 3;
const POLL_MS: u64 = 100;

pub fn run() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();
    app.reload();

    let mut last_auto_refresh = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        match next_event(POLL_MS)? {
            Some(AppEvent::Key(key)) => {
                if events::handle_key(&mut app, key, terminal)? {
                    break;
                }
            }
            Some(AppEvent::Tick) => {
                if last_auto_refresh.elapsed() >= Duration::from_secs(REFRESH_INTERVAL_SECS) {
                    app.reload();
                    last_auto_refresh = Instant::now();
                }
            }
            None => {
                app.reload();
                last_auto_refresh = Instant::now();
            }
        }
    }

    Ok(())
}
