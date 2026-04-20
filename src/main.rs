use tracing::info;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("claude-rl-daemon {VERSION}");
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "claude_rl_daemon=info".into()),
        )
        .json()
        .init();

    info!(version = VERSION, "claude-rl-daemon starting");

    tokio::select! {
        result = claude_rl_daemon::watcher::run() => result,
        _ = tokio::signal::ctrl_c() => {
            info!("received SIGINT, shutting down");
            Ok(())
        }
    }
}
