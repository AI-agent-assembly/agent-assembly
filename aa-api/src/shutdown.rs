//! Graceful shutdown signal handler.
//!
//! Listens for `SIGTERM` (and `Ctrl-C` for dev convenience) and returns
//! a future that completes when the signal is received. The server uses
//! this to drain in-flight requests within a configurable timeout.

use std::time::Duration;

/// Default drain timeout after receiving a shutdown signal.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// Returns a future that completes when a shutdown signal is received.
///
/// On Unix, listens for both `SIGTERM` and `SIGINT` (Ctrl-C).
pub async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("received SIGINT, starting graceful shutdown");
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, starting graceful shutdown");
            }
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("failed to listen for Ctrl-C");
        tracing::info!("received Ctrl-C, starting graceful shutdown");
    }
}
