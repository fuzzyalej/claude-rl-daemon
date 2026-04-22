use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::Cli;

#[cfg(not(tarpaulin))]
pub fn run(shell: Shell) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "cdaemon", &mut std::io::stdout());
    Ok(())
}
