//! Health check and Prometheus metrics HTTP server.

use std::sync::atomic::AtomicI64;
use std::sync::Arc;

use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::ipc::message::IpcFrame;
use crate::pipeline::PipelineMetrics;

/// Shared state passed to all axum handlers.
#[derive(Clone)]
pub struct HealthState {
    pub start_time: std::time::Instant,
    pub pipeline_metrics: Arc<PipelineMetrics>,
    pub ready_rx: watch::Receiver<bool>,
    pub prometheus_handle: PrometheusHandle,
    pub active_connections: Arc<AtomicI64>,
    pub inbound_tx: mpsc::Sender<IpcFrame>,
}

/// Response body for GET /health.
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub uptime_secs: u64,
    pub events_processed: u64,
}

/// Build the axum router with all three routes.
pub fn router(state: HealthState) -> Router {
    Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/ready", axum::routing::get(ready_handler))
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(state)
}

// --- Stub handlers (replaced in Tasks 6–8) ---

async fn health_handler(axum::extract::State(state): axum::extract::State<HealthState>) -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse {
        status: "healthy",
        uptime_secs: state.start_time.elapsed().as_secs(),
        events_processed: state.pipeline_metrics.processed(),
    })
}

async fn ready_handler(axum::extract::State(state): axum::extract::State<HealthState>) -> axum::response::Response {
    use axum::response::IntoResponse;
    if *state.ready_rx.borrow() {
        (axum::http::StatusCode::OK, "ready").into_response()
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}

async fn metrics_handler(axum::extract::State(_state): axum::extract::State<HealthState>) -> String {
    todo!("implemented in Task 8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    fn make_prometheus_handle() -> metrics_exporter_prometheus::PrometheusHandle {
        metrics_exporter_prometheus::PrometheusBuilder::new()
            .build_recorder()
            .handle()
    }

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

    #[tokio::test]
    async fn health_endpoint_returns_200_with_json() {
        let (_, ready_rx) = tokio::sync::watch::channel(false);
        let (inbound_tx, _) = tokio::sync::mpsc::channel(1);
        let pipeline_metrics = Arc::new(crate::pipeline::PipelineMetrics::default());

        let state = HealthState {
            start_time: std::time::Instant::now(),
            pipeline_metrics,
            ready_rx,
            prometheus_handle: make_prometheus_handle(),
            active_connections: Arc::new(AtomicI64::new(0)),
            inbound_tx,
        };

        let app = router(state);
        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
        assert!(json["uptime_secs"].as_u64().is_some());
        assert_eq!(json["events_processed"], 0);
    }

    #[tokio::test]
    async fn ready_returns_503_when_not_ready() {
        let (_, ready_rx) = tokio::sync::watch::channel(false);
        let (inbound_tx, _) = tokio::sync::mpsc::channel(1);
        let pipeline_metrics = Arc::new(crate::pipeline::PipelineMetrics::default());

        let state = HealthState {
            start_time: std::time::Instant::now(),
            pipeline_metrics,
            ready_rx,
            prometheus_handle: make_prometheus_handle(),
            active_connections: Arc::new(AtomicI64::new(0)),
            inbound_tx,
        };

        let app = router(state);
        let req = Request::builder().uri("/ready").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn ready_returns_200_when_ready() {
        let (_, ready_rx) = tokio::sync::watch::channel(true);
        let (inbound_tx, _) = tokio::sync::mpsc::channel(1);
        let pipeline_metrics = Arc::new(crate::pipeline::PipelineMetrics::default());

        let state = HealthState {
            start_time: std::time::Instant::now(),
            pipeline_metrics,
            ready_rx,
            prometheus_handle: make_prometheus_handle(),
            active_connections: Arc::new(AtomicI64::new(0)),
            inbound_tx,
        };

        let app = router(state);
        let req = Request::builder().uri("/ready").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_reflects_watch_channel_update() {
        let (ready_tx, ready_rx) = tokio::sync::watch::channel(false);
        let (inbound_tx, _) = tokio::sync::mpsc::channel(1);
        let pipeline_metrics = Arc::new(crate::pipeline::PipelineMetrics::default());

        let state = HealthState {
            start_time: std::time::Instant::now(),
            pipeline_metrics,
            ready_rx,
            prometheus_handle: make_prometheus_handle(),
            active_connections: Arc::new(AtomicI64::new(0)),
            inbound_tx,
        };

        // First request: not ready (503)
        let app1 = router(state.clone());
        let req1 = Request::builder().uri("/ready").body(Body::empty()).unwrap();
        let response1 = app1.oneshot(req1).await.unwrap();
        assert_eq!(response1.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Update watch channel to ready
        ready_tx.send(true).unwrap();

        // Second request: now ready (200)
        let app2 = router(state.clone());
        let req2 = Request::builder().uri("/ready").body(Body::empty()).unwrap();
        let response2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(response2.status(), StatusCode::OK);
    }
}
