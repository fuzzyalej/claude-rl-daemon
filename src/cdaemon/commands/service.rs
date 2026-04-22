use std::path::Path;
use std::process::Command;

use anyhow::Context;
use colored::Colorize;

use crate::state;

const PLIST_TEMPLATE: &str = include_str!("../../../deploy/com.claudedaemon.plist");

#[cfg(not(tarpaulin))]
pub fn install() -> anyhow::Result<()> {
    println!("Building release binaries...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .context("failed to run cargo build")?;
    anyhow::ensure!(status.success(), "cargo build --release failed");

    let install_dir = dirs::home_dir().unwrap().join(".local/bin");
    std::fs::create_dir_all(&install_dir)?;

    for name in &["claude-rl-daemon", "cdaemon"] {
        let src = Path::new("target/release").join(name);
        let dst = install_dir.join(name);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("failed to copy {name} — run from project root"))?;
        println!("{} installed {}", "✓".green(), dst.display());
    }

    let log_dir = dirs::home_dir().unwrap().join(".claude-daemon");
    std::fs::create_dir_all(&log_dir)?;

    let install_path = install_dir.join("claude-rl-daemon");
    let home = dirs::home_dir().unwrap();
    let plist_content = PLIST_TEMPLATE
        .replace("__INSTALL_PATH__", install_path.to_str().unwrap())
        .replace("__HOME__", home.to_str().unwrap());

    let plist_dst = state::plist_path();
    std::fs::create_dir_all(plist_dst.parent().unwrap())?;
    std::fs::write(&plist_dst, plist_content)?;
    println!("{} plist installed to {}", "✓".green(), plist_dst.display());

    let _ = Command::new("launchctl")
        .args(["unload", plist_dst.to_str().unwrap()])
        .status();

    let status = Command::new("launchctl")
        .args(["load", plist_dst.to_str().unwrap()])
        .status()
        .context("failed to run launchctl load")?;
    anyhow::ensure!(status.success(), "launchctl load failed");

    println!("{} Daemon installed and running.", "✓".green());
    println!();
    println!("  Check status:  cdaemon status");
    println!("  View logs:     cdaemon logs --follow");
    println!("  Stop daemon:   cdaemon stop");
    Ok(())
}

#[cfg(not(tarpaulin))]
pub fn start() -> anyhow::Result<()> {
    let plist = state::plist_path();
    anyhow::ensure!(
        plist.exists(),
        "plist not found at {}. Run 'cdaemon install' first.",
        plist.display()
    );
    let status = Command::new("launchctl")
        .args(["load", plist.to_str().unwrap()])
        .status()?;
    anyhow::ensure!(status.success(), "launchctl load failed");
    println!("{} Daemon started.", "✓".green());
    Ok(())
}

#[cfg(not(tarpaulin))]
pub fn stop() -> anyhow::Result<()> {
    let plist = state::plist_path();
    let status = Command::new("launchctl")
        .args(["unload", plist.to_str().unwrap()])
        .status()?;
    if status.success() {
        println!("{} Daemon stopped.", "✓".green());
    } else {
        println!("{}", "— Daemon was not running.".dimmed());
    }
    Ok(())
}

#[cfg(not(tarpaulin))]
pub fn uninstall() -> anyhow::Result<()> {
    stop().ok();

    let plist = state::plist_path();
    if plist.exists() {
        std::fs::remove_file(&plist)?;
        println!("{} Removed plist.", "✓".green());
    }

    let daemon_bin = state::daemon_bin_path();
    if daemon_bin.exists() {
        std::fs::remove_file(&daemon_bin)?;
        println!("{} Removed daemon binary.", "✓".green());
    }

    println!("State and logs in ~/.claude-daemon/ were kept.");
    Ok(())
}
