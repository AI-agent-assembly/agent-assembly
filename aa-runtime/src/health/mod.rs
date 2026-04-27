//! Health check and Prometheus metrics HTTP server.

use std::sync::Arc;
use std::sync::atomic::AtomicI64;

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio::sync::watch;
use tokio::sync::mpsc;

use crate::ipc::message::IpcFrame;
use crate::pipeline::PipelineMetrics;

/// Shared state passed to all axum handlers.
#[derive(Clone)]
pub struct HealthState {
    pub start_time:         std::time::Instant,
    pub pipeline_metrics:   Arc<PipelineMetrics>,
    pub ready_rx:           watch::Receiver<bool>,
    pub prometheus_handle:  PrometheusHandle,
    pub active_connections: Arc<AtomicI64>,
    pub inbound_tx:         mpsc::Sender<IpcFrame>,
}

/// Response body for GET /health.
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status:           &'static str,
    pub uptime_secs:      u64,
    pub events_processed: u64,
}

/// Build the axum router with all three routes.
pub fn router(state: HealthState) -> Router {
    Router::new()
        .route("/health",  axum::routing::get(health_handler))
        .route("/ready",   axum::routing::get(ready_handler))
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(state)
}

// --- Stub handlers (replaced in Tasks 6–8) ---

async fn health_handler(
    axum::extract::State(_state): axum::extract::State<HealthState>,
) -> axum::Json<HealthResponse> {
    todo!("implemented in Task 6")
}

async fn ready_handler(
    axum::extract::State(_state): axum::extract::State<HealthState>,
) -> axum::response::Response {
    todo!("implemented in Task 7")
}

async fn metrics_handler(
    axum::extract::State(_state): axum::extract::State<HealthState>,
) -> String {
    todo!("implemented in Task 8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes_correctly() {
        let resp = HealthResponse {
            status: "healthy",
            uptime_secs: 42,
            events_processed: 100,
        };
        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"uptime_secs\":42"));
        assert!(json.contains("\"events_processed\":100"));
    }
}
