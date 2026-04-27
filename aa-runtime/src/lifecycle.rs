//! Signal handling and graceful shutdown coordination.

/// Waits until the process receives a shutdown signal (SIGTERM or SIGINT).
///
/// Returns as soon as either signal fires. Callers should then trigger
/// cooperative cancellation on all tracked tasks.
pub async fn wait_for_shutdown_signal() {
    // SIGTERM and SIGINT handlers added in subsequent steps.
    std::future::pending::<()>().await
}
