use anyhow::Result;

#[cfg(target_os = "macos")]
pub fn notify(title: &str, body: &str) -> Result<()> {
    use std::process::Command;
    use tracing::info;

    // Escape double quotes so the AppleScript string is valid
    let esc_title = title.replace('"', "\\\"");
    let esc_body = body.replace('"', "\\\"");
    let script = format!("display notification \"{}\" with title \"{}\"", esc_body, esc_title);

    // Run osascript -e '<script>'
    let status = Command::new("osascript").arg("-e").arg(script).status()?;
    if status.success() {
        info!(title = %title, body = %body, "macOS notification delivered");
        Ok(())
    } else {
        anyhow::bail!("osascript exited with status {:?}", status.code())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn notify(title: &str, body: &str) -> Result<()> {
    // No-op on non-macOS, but keep a log entry so users know notifications are skipped
    tracing::info!(title = %title, body = %body, "notification (noop on non-macos)");
    Ok(())
}
