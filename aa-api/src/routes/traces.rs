//! Agent session trace endpoints.

use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::ProblemDetail;
use crate::state::AppState;

/// A single span within an agent session trace.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TraceSpan {
    /// Span identifier.
    pub span_id: String,
    /// ISO 8601 start time.
    pub start_time: String,
    /// ISO 8601 end time (if completed).
    pub end_time: Option<String>,
    /// Operation name.
    pub operation: String,
    /// Governance decision result for this span.
    pub decision: Option<String>,
}

/// Full trace for one agent session.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TraceResponse {
    /// Session identifier.
    pub session_id: String,
    /// Agent that produced this trace.
    pub agent_id: String,
    /// Ordered list of spans in the session.
    pub spans: Vec<TraceSpan>,
}

/// `GET /api/v1/traces/:session_id` — full trace for one agent session.
#[utoipa::path(
    get,
    path = "/api/v1/traces/{session_id}",
    params(("session_id" = String, Path, description = "Agent session identifier")),
    responses(
        (status = 200, description = "Session trace", body = TraceResponse),
        (status = 404, description = "Session not found")
    ),
    tag = "traces"
)]
pub async fn get_trace(
    Extension(_state): Extension<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<TraceResponse>), ProblemDetail> {
    // TODO: wire to trace store once available
    Err(ProblemDetail::from_status(StatusCode::NOT_FOUND)
        .with_detail(format!("Session not found: {session_id}")))
}
