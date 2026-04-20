use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "claude_rl_daemon=info".into()),
        )
        .json()
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "claude-rl-daemon starting"
    );

    tokio::select! {
        result = claude_rl_daemon::watcher::run() => result,
        _ = tokio::signal::ctrl_c() => {
            info!("received SIGINT, shutting down");
            Ok(())
        }
    }
}
