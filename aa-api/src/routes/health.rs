//! Health check endpoint.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::error::ProblemDetail;

/// Response body for the health endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Liveness status string, always `"ok"` when the service is running.
    pub status: String,
}

/// `GET /api/v1/health` — liveness probe.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 404, description = "Not found", body = ProblemDetail)
    )
)]
pub async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
        }),
    )
}
