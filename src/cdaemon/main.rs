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
    command: Cmd,
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
    /// Cancel a pending session resume
    Cancel {
        /// Session UUID or 8-char prefix
        uuid: String,
    },
    /// Check all prerequisites
    Doctor,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Status => commands::status::run(),
        Cmd::Sessions => commands::sessions::run(),
        Cmd::Logs { follow, lines } => commands::logs::run(follow, lines),
        Cmd::Install => commands::service::install(),
        Cmd::Start => commands::service::start(),
        Cmd::Stop => commands::service::stop(),
        Cmd::Uninstall => commands::service::uninstall(),
        Cmd::Hook { uuid } => commands::hook::run(&uuid),
        Cmd::Resume { uuid } => commands::resume::run(&uuid),
        Cmd::Reschedule { uuid, time } => commands::reschedule::run(&uuid, &time),
        Cmd::Cancel { uuid } => commands::cancel::run(&uuid),
        Cmd::Doctor => commands::doctor::run(),
        Cmd::Completions { shell } => commands::completions::run(shell),
    }
}
