mod commands;
mod format;
mod state;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "cdaemon", version, about = "Manage claude-rl-daemon sessions and service")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show daemon status and pending sessions
    Status,
    /// List all sessions with details
    Sessions,
    /// View daemon logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },
    /// Install daemon binary and launchd service
    Install,
    /// Start the daemon via launchd
    Start,
    /// Stop the daemon via launchd
    Stop,
    /// Uninstall the daemon service and binary
    Uninstall,
    /// Attach to a session's tmux window
    Hook {
        /// Session UUID or 8-char prefix
        uuid: String,
    },
    /// Manually resume a session now
    Resume {
        /// Session UUID or 8-char prefix
        uuid: String,
    },
    /// Reschedule a pending session's resume time
    Reschedule {
        /// Session UUID or 8-char prefix
        uuid: String,
        /// New resume time (ISO8601 or relative, e.g. "+2h", "in 10m")
        time: String,
    },
    /// Cancel pending session resumes
    Cancel {
        /// Session UUID(s), 8-char prefix(es), or 'all' to clear everything
        #[arg(num_args = 1..)]
        uuids: Vec<String>,
    },
    /// Check all prerequisites
    Doctor,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[cfg(tarpaulin)]
fn main() {}

#[cfg(not(tarpaulin))]
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => commands::tui::run(),
        Some(Cmd::Status) => commands::status::run(),
        Some(Cmd::Sessions) => commands::sessions::run(),
        Some(Cmd::Logs { follow, lines }) => commands::logs::run(follow, lines),
        Some(Cmd::Install) => commands::service::install(),
        Some(Cmd::Start) => commands::service::start(),
        Some(Cmd::Stop) => commands::service::stop(),
        Some(Cmd::Uninstall) => commands::service::uninstall(),
        Some(Cmd::Hook { uuid }) => commands::hook::run(&uuid),
        Some(Cmd::Resume { uuid }) => commands::resume::run(&uuid),
        Some(Cmd::Reschedule { uuid, time }) => commands::reschedule::run(&uuid, &time),
        Some(Cmd::Cancel { uuids }) => commands::cancel::run(&uuids),
        Some(Cmd::Doctor) => commands::doctor::run(),
        Some(Cmd::Completions { shell }) => commands::completions::run(shell),
    }
}
