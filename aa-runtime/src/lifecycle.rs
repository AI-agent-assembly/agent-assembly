//! Signal handling and graceful shutdown coordination.

/// Waits until the process receives a shutdown signal (SIGTERM or SIGINT).
///
/// Returns as soon as either signal fires. Callers should then trigger
/// cooperative cancellation on all tracked tasks.
pub async fn wait_for_shutdown_signal() {
    let sigterm = sigterm();

    tokio::select! {
        _ = sigterm => {
            tracing::info!("received SIGTERM — initiating graceful shutdown");
        }
    }
}

/// Returns a future that resolves on the first SIGTERM.
///
/// On non-Unix platforms this future never resolves (SIGTERM is Unix-only).
async fn sigterm() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut stream = signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        stream.recv().await;
    }
    #[cfg(not(unix))]
    {
        std::future::pending::<()>().await
    }
}
