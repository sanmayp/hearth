//! agent-bridge binary. It serves herdr panes on loopback.
//! It refuses start without a token file.

use std::net::SocketAddr;
use std::sync::Arc;

use agent_bridge::{build_router, AppState, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("agent_bridge=info,tower_http=info")
        .json()
        .init();
    let config = match Config::load_from_env() {
        Ok(c) => c,
        Err(e) => {
            // Never print the token. The error holds paths only.
            eprintln!("agent-bridge refuses to start: {e}");
            std::process::exit(1);
        }
    };
    let port = config.port;
    let state = Arc::new(AppState::new(config));
    let app = build_router(state);
    // Loopback only. The tailnet proxy fronts it. No other bind exists.
    let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
    tracing::info!(%addr, "agent-bridge serves");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// SIGTERM/SIGINT drain. It stops accept and ends in flight.
async fn shutdown() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM hook");
    let mut int = signal(SignalKind::interrupt()).expect("SIGINT hook");
    tokio::select! {
        _ = term.recv() => tracing::info!("SIGTERM: drain"),
        _ = int.recv() => tracing::info!("SIGINT: drain"),
    }
}
