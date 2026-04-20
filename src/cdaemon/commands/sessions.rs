use colored::Colorize;
use tabled::{Table, Tabled};

use crate::{format, state};

#[derive(Tabled)]
struct SessionRow {
    #[tabled(rename = "UUID")]
    uuid: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Reset At")]
    reset_at: String,
    #[tabled(rename = "CWD")]
    cwd: String,
}

pub fn run() -> anyhow::Result<()> {
    let daemon_state = state::load_state()?;
    let mut rows: Vec<SessionRow> = Vec::new();

    for resume in daemon_state.pending.values() {
        let r = format::session_row(resume, "pending");
        rows.push(SessionRow { uuid: r.uuid, status: r.status, reset_at: r.reset_at, cwd: r.cwd });
    }

    for id in &daemon_state.completed {
        rows.push(SessionRow {
            uuid: id.clone(),
            status: format::color_status("resumed"),
            reset_at: "—".to_string(),
            cwd: "—".to_string(),
        });
    }

    if rows.is_empty() {
        println!("{}", "No sessions recorded.".dimmed());
        return Ok(());
    }

    println!("{}", Table::new(rows));
    Ok(())
}
