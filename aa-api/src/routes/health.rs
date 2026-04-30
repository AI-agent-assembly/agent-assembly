//! Health check endpoint.

use std::sync::atomic::Ordering;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Serialize;

use crate::state::AppState;

/// Response body for the health endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Liveness status string, always `"ok"` when the service is running.
    pub status: String,
    /// Server uptime in seconds since startup.
    pub uptime_secs: u64,
    /// Number of currently active WebSocket/SSE connections.
    pub active_connections: i64,
    /// Pipeline processing lag in milliseconds (placeholder, always 0 for now).
    pub pipeline_lag_ms: u64,
}

/// `GET /api/v1/health` — liveness probe.
///
/// Returns a simple JSON body indicating the service is alive.
/// Suitable for Kubernetes liveness probes.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 404, description = "Not found", body = ProblemDetail)
    )
)]
pub async fn health(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let uptime_secs = state.startup_time.elapsed().as_secs();
    let active_connections = state.active_connections.load(Ordering::Relaxed);

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            uptime_secs,
            active_connections,
            pipeline_lag_ms: 0,
        }),
    )
}
