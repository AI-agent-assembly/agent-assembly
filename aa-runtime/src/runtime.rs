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
    // Install global Prometheus metrics recorder.
    let prometheus_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    // Register all 6 required metrics at 0 so /metrics surface is stable from first scrape.
    metrics::counter!("aa_events_received_total").increment(0);
    metrics::counter!("aa_events_emitted_total").increment(0);
    metrics::counter!("aa_policy_violations_total").increment(0);
    metrics::counter!("aa_policy_evaluations_total").increment(0); // stays 0 until AAASM-69/70
    metrics::gauge!("aa_active_connections").set(0.0);
    metrics::gauge!("aa_channel_utilization_ratio").set(0.0);

    // Readiness channel — written true after IpcServer::bind() succeeds.
    let (ready_tx, ready_rx) = tokio::sync::watch::channel(false);

    tracing::info!("aa-runtime starting");

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    tracing::info!("structured concurrency primitives initialised");

    // Build pipeline config and create the inbound channel at the configured depth.
    let pipeline_config = crate::pipeline::PipelineConfig::from_runtime_config(&config);
    let (inbound_tx, inbound_rx) = tokio::sync::mpsc::channel::<crate::ipc::IpcFrame>(pipeline_config.input_buffer);

    // Create the broadcast channel for fan-out to downstream subscribers.
    // The leading `_broadcast_rx` keeps the channel alive until real subscribers
    // are wired in AAASM-32+.
    let (broadcast_tx, _broadcast_rx) =
        tokio::sync::broadcast::channel::<crate::pipeline::EnrichedEvent>(pipeline_config.broadcast_capacity);

    // Shared metrics — future health/metrics endpoints will receive an Arc clone.
    let pipeline_metrics = std::sync::Arc::new(crate::pipeline::PipelineMetrics::default());

    // Shared active-connections counter exposed to the health/metrics endpoint.
    let active_connections = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));

    // Clone inbound_tx for the health/metrics handler before IpcServer consumes it.
    let inbound_tx_health = inbound_tx.clone();

    // Spawn the IPC server task.
    let ipc_config = crate::ipc::server::IpcServerConfig::from_runtime_config(&config);
    match crate::ipc::server::IpcServer::bind(ipc_config) {
        Ok(ipc_server) => {
            let _ = ready_tx.send(true);
            let ipc_tracker = tracker.clone();
            let ipc_token = token.clone();
            let ipc_active_connections = std::sync::Arc::clone(&active_connections);
            tracker.spawn(async move {
                ipc_server
                    .run(ipc_tracker, ipc_token, inbound_tx, ipc_active_connections)
                    .await;
            });
            tracing::info!("IPC server task spawned");
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to bind IPC socket — continuing without IPC");
            // Without an IPC server the inbound_tx is dropped here;
            // the pipeline will see the channel closed and exit cleanly.
        }
    }

    // Spawn the event aggregation pipeline task.
    {
        let pipeline_token = token.clone();
        let pm = pipeline_metrics.clone();
        tracker.spawn(async move {
            crate::pipeline::run(inbound_rx, broadcast_tx, pipeline_config, pm, pipeline_token).await;
        });
        tracing::info!("pipeline task spawned");
    }

    // Spawn the health/metrics HTTP server task.
    {
        let health_state = crate::health::HealthState {
            start_time: std::time::Instant::now(),
            pipeline_metrics: std::sync::Arc::clone(&pipeline_metrics),
            ready_rx,
            prometheus_handle,
            active_connections: std::sync::Arc::clone(&active_connections),
            inbound_tx: inbound_tx_health,
        };
        let addr: std::net::SocketAddr = config.metrics_addr
            .parse()
            .expect("invalid AA_METRICS_ADDR — must be a valid socket address");
        let health_token = token.clone();
        tracker.spawn(async move {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    tracing::info!(%addr, "health server bound");
                    axum::serve(listener, crate::health::router(health_state))
                        .with_graceful_shutdown(async move { health_token.cancelled().await })
                        .await
                        .ok();
                }
                Err(e) => {
                    tracing::error!(error = %e, %addr, "failed to bind health server");
                }
            }
        });
        tracing::info!(%addr, "health server task spawned");
    }

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
