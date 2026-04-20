use std::process::Command;

use colored::Colorize;

use crate::state;

struct Check {
    label: &'static str,
    passed: bool,
    hint: &'static str,
}

pub fn run() -> anyhow::Result<()> {
    let checks = vec![
        Check {
            label: "tmux on PATH",
            passed: cmd_exists("tmux"),
            hint: "brew install tmux",
        },
        Check {
            label: "daemon binary (~/.local/bin/claude-rl-daemon)",
            passed: state::daemon_bin_path().exists(),
            hint: "run: cdaemon install",
        },
        Check {
            label: "cdaemon binary (~/.local/bin/cdaemon)",
            passed: state::cdaemon_bin_path().exists(),
            hint: "run: cdaemon install",
        },
        Check {
            label: "launchd plist (~/Library/LaunchAgents/com.claudedaemon.plist)",
            passed: state::plist_path().exists(),
            hint: "run: cdaemon install",
        },
        Check {
            label: "launchd label com.claudedaemon loaded",
            passed: launchd_loaded(),
            hint: "run: cdaemon start",
        },
        Check {
            label: "sessions dir (~/.claude/projects/)",
            passed: dirs::home_dir()
                .map(|h| h.join(".claude/projects").exists())
                .unwrap_or(false),
            hint: "open Claude Code at least once",
        },
        Check {
            label: "state dir (~/.claude-daemon/)",
            passed: dirs::home_dir()
                .map(|h| h.join(".claude-daemon").exists())
                .unwrap_or(false),
            hint: "run: cdaemon start",
        },
    ];

    let all_passed = checks.iter().all(|c| c.passed);

    for check in &checks {
        if check.passed {
            println!("  {}  {}", "✓".green(), check.label);
        } else {
            println!("  {}  {}  → {}", "✗".red(), check.label, check.hint.dimmed());
        }
    }

    println!();
    if all_passed {
        println!("{}", "All checks passed.".green());
    } else {
        println!("{}", "Some checks failed. See hints above.".yellow());
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn launchd_loaded() -> bool {
    Command::new("launchctl")
        .args(["list", "com.claudedaemon"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
