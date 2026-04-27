//! Tokio runtime initialisation and structured task lifecycle management.

use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::config::RuntimeConfig;
use crate::lifecycle::wait_for_shutdown_signal;

/// Start the runtime and block until graceful shutdown completes.
///
/// This is the main async entry point called from `main()`. It creates the
/// structured concurrency primitives, spawns subsystem tasks, waits for a
/// shutdown signal, then drains all tasks within the configured timeout.
pub async fn run(config: RuntimeConfig) {
    tracing::info!("aa-runtime starting");

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    tracing::info!("structured concurrency primitives initialised");

    // Subsystem tasks (ipc, pipeline, health) are spawned here in later tickets.
    // Each receives a cloned `token` and a `tracker.token()` guard.

    // Wait for an OS shutdown signal.
    wait_for_shutdown_signal().await;

    // Signal all tasks to stop cooperatively.
    token.cancel();
    tracing::info!("cancellation token fired — draining tasks");

    // Stop accepting new task registrations.
    tracker.close();

    // Wait for all tasks to complete, with a hard timeout.
    let timeout = Duration::from_secs(config.shutdown_timeout_secs);
    if tokio::time::timeout(timeout, tracker.wait()).await.is_err() {
        tracing::error!(
            timeout_secs = config.shutdown_timeout_secs,
            "shutdown timeout exceeded — forcing exit"
        );
    } else {
        tracing::info!("all tasks completed cleanly");
    }

    tracing::info!("aa-runtime stopped");
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;
    use tokio_util::task::TaskTracker;

    /// Verifies the structured concurrency primitives drain cleanly under load.
    ///
    /// Spawns N tasks that loop until the cancellation token fires, then
    /// cancels the token and asserts all tasks complete within the timeout.
    #[tokio::test]
    async fn graceful_shutdown_drains_all_tasks() {
        const TASK_COUNT: usize = 10;
        const TIMEOUT: Duration = Duration::from_secs(5);

        let tracker = TaskTracker::new();
        let token = CancellationToken::new();

        // Spawn synthetic load tasks that honor the cancellation token.
        for i in 0..TASK_COUNT {
            let child_token = token.clone();
            tracker.spawn(async move {
                loop {
                    tokio::select! {
                        _ = child_token.cancelled() => {
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {
                            // Simulate work.
                        }
                    }
                }
                tracing::debug!(task = i, "task completed cleanly");
            });
        }

        // Trigger shutdown.
        token.cancel();
        tracker.close();

        // All tasks must complete within the timeout — no leaks.
        tokio::time::timeout(TIMEOUT, tracker.wait())
            .await
            .expect("tasks did not complete within timeout");
    }

    /// Verifies that shutdown timeout enforcement works when tasks ignore cancellation.
    #[tokio::test]
    async fn shutdown_timeout_fires_when_tasks_hang() {
        let tracker = TaskTracker::new();
        let token = CancellationToken::new();

        // Spawn a task that ignores cancellation and sleeps forever.
        tracker.spawn(async move {
            let _token = token; // hold token to prevent drop-based cancellation
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });

        tracker.close();

        // Drain with a very short timeout — must expire.
        let result = tokio::time::timeout(Duration::from_millis(100), tracker.wait()).await;
        assert!(result.is_err(), "expected timeout but tasks completed");
    }
}
