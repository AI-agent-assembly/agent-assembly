//! Tokio runtime initialisation and structured task lifecycle management.

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::config::RuntimeConfig;

/// Start the runtime and block until graceful shutdown completes.
///
/// This is the main async entry point called from `main()`. It creates the
/// structured concurrency primitives, spawns subsystem tasks, waits for a
/// shutdown signal, then drains all tasks within the configured timeout.
pub async fn run(_config: RuntimeConfig) {
    tracing::info!("aa-runtime starting");

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    tracing::info!(
        tasks = tracker.len(),
        "structured concurrency primitives initialised"
    );

    // Shutdown sequence added in the next task.
    drop(token);
    tracker.close();
    tracker.wait().await;

    tracing::info!("aa-runtime stopped");
}
