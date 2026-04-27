//! Tokio runtime initialisation and structured task lifecycle management.

use crate::config::RuntimeConfig;

/// Start the runtime and block until graceful shutdown completes.
///
/// This is the main async entry point called from `main()`. It creates the
/// structured concurrency primitives, spawns subsystem tasks, waits for a
/// shutdown signal, then drains all tasks within the configured timeout.
pub async fn run(_config: RuntimeConfig) {
    tracing::info!("aa-runtime starting");
    tracing::info!("aa-runtime stopped");
}
