//! Audit log query endpoints.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::pagination::{PaginatedResponse, PaginationParams};
use crate::state::AppState;

/// JSON representation of an audit log entry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogEntry {
    /// Unique log entry identifier.
    pub id: String,
    /// ISO 8601 timestamp of the event.
    pub timestamp: String,
    /// Agent ID that produced this log entry.
    pub agent_id: String,
    /// Session ID for the agent run.
    pub session_id: String,
    /// Type of audit event.
    pub event_type: String,
    /// Human-readable summary of the event.
    pub summary: String,
}

/// `GET /api/v1/logs` — paginated audit log query.
///
/// Query the paginated audit log of governance events.
#[utoipa::path(
    get,
    path = "/api/v1/logs",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated audit log entries", body = Vec<LogEntry>)
    ),
    tag = "logs"
)]
pub async fn list_logs(
    Extension(_state): Extension<AppState>,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> impl IntoResponse {
    // TODO: wire to audit store once available
    let items: Vec<LogEntry> = Vec::new();

    (
        StatusCode::OK,
        Json(PaginatedResponse {
            items,
            page: params.page(),
            per_page: params.per_page(),
            total: 0,
        }),
    )
}
