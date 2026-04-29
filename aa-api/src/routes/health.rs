//! Health check endpoint.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

/// Response body for the health endpoint.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// `GET /api/v1/health` — liveness probe.
pub async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
        }),
    )
}
