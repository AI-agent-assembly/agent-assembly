//! Agent session trace endpoints.

use axum::http::StatusCode;
use axum::{Extension, Json};

use crate::error::ProblemDetail;
use crate::models::trace::TraceResponse;
use crate::state::AppState;

/// `GET /api/v1/traces/:session_id` — full trace for one agent session.
///
/// Retrieve the full ordered trace of spans for one agent session.
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
    Extension(state): Extension<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<TraceResponse>), ProblemDetail> {
    let trace = state
        .trace_store
        .get_trace(&session_id)
        .map_err(|e| ProblemDetail::from_status(StatusCode::INTERNAL_SERVER_ERROR).with_detail(e.to_string()))?;

    match trace {
        Some(session_trace) => Ok((
            StatusCode::OK,
            Json(TraceResponse {
                session_id,
                agent_id: session_trace.agent_id,
                spans: session_trace.spans,
            }),
        )),
        None => {
            Err(ProblemDetail::from_status(StatusCode::NOT_FOUND)
                .with_detail(format!("Session not found: {session_id}")))
        }
    }
}
