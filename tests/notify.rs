#[cfg(test)]
mod tests {
    use claude_rl_daemon::notify;

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn notify_noop_on_non_macos() {
        notify::notify("test", "body").expect("notify should not error on non-macos");
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn macos_notify_smoke() {
        // Ignored by default because it invokes osascript on the host machine.
        notify::notify("test", "body").expect("osascript should run (ignored by default)");
    }
}
