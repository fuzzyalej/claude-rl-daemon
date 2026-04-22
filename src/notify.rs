use anyhow::Result;

#[cfg(all(target_os = "macos", not(tarpaulin)))]
pub fn notify(title: &str, body: &str) -> Result<()> {
    use std::process::Command;
    use tracing::info;

    let esc_title = title.replace('"', "\\\"");
    let esc_body = body.replace('"', "\\\"");
    let script = format!("display notification \"{}\" with title \"{}\"", esc_body, esc_title);

    let status = Command::new("osascript").arg("-e").arg(script).status()?;
    if status.success() {
        info!(title = %title, body = %body, "macOS notification delivered");
        Ok(())
    } else {
        anyhow::bail!("osascript exited with status {:?}", status.code())
    }
}

#[cfg(any(not(target_os = "macos"), tarpaulin))]
pub fn notify(title: &str, body: &str) -> Result<()> {
    tracing::info!(title = %title, body = %body, "notification (noop on non-macos)");
    Ok(())
}
