use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use anyhow::Context;
use colored::Colorize;

use crate::state;

#[cfg(not(tarpaulin))]
pub fn run(follow: bool, lines: usize) -> anyhow::Result<()> {
    let log_path = state::log_path();

    if !log_path.exists() {
        println!("{}", "Log file not found. Has the daemon run yet?".yellow());
        println!("Expected: {}", log_path.display());
        return Ok(());
    }

    if follow {
        let mut child = Command::new("tail")
            .args(["-f", log_path.to_str().unwrap()])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to spawn tail")?;
        child.wait()?;
        return Ok(());
    }

    let file = std::fs::File::open(&log_path).context("failed to open daemon.log")?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    let start = all_lines.len().saturating_sub(lines);

    for line in &all_lines[start..] {
        println!("{line}");
    }

    Ok(())
}
