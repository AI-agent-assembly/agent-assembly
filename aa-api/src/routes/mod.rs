//! Route definitions for the REST API.
//!
//! All endpoints are nested under `/api/v1/`.

pub mod auth;
pub mod health;

use axum::routing::{get, post};
use axum::Router;

use crate::error::ProblemDetail;

/// Build the v1 API router with all registered routes.
pub fn v1_router() -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ws/events", get(crate::ws::handler::ws_events_handler))
        .route("/auth/token", post(auth::issue_token))
}

/// Fallback handler returning a 404 RFC 7807 response.
pub async fn fallback_404(uri: axum::http::Uri) -> ProblemDetail {
    ProblemDetail::from_status(axum::http::StatusCode::NOT_FOUND)
        .with_detail(format!("No route matched: {uri}"))
        .with_instance(uri.to_string())
}
