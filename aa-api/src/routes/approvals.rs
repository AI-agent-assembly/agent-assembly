//! Human-in-the-loop approval endpoints.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::ProblemDetail;
use crate::pagination::{PaginatedResponse, PaginationParams};
use crate::state::AppState;

/// JSON representation of a pending approval request.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApprovalResponse {
    /// Unique approval request identifier.
    pub id: String,
    /// Agent that triggered the approval.
    pub agent_id: String,
    /// The governance action requiring approval.
    pub action: String,
    /// Human-readable reason for the approval request.
    pub reason: String,
    /// Current status: "pending", "approved", or "rejected".
    pub status: String,
    /// ISO 8601 timestamp when the request was created.
    pub created_at: String,
}

/// `GET /api/v1/approvals` — list pending approval requests.
#[utoipa::path(
    get,
    path = "/api/v1/approvals",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of pending approvals")
    ),
    tag = "approvals"
)]
pub async fn list_approvals(
    Extension(_state): Extension<AppState>,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> impl IntoResponse {
    // TODO: wire to approval queue once query interface is available
    let items: Vec<ApprovalResponse> = Vec::new();

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

/// `POST /api/v1/approvals/:id/approve` — approve a pending action.
#[utoipa::path(
    post,
    path = "/api/v1/approvals/{id}/approve",
    params(("id" = String, Path, description = "Approval request identifier")),
    responses(
        (status = 200, description = "Action approved", body = ApprovalResponse),
        (status = 404, description = "Approval request not found")
    ),
    tag = "approvals"
)]
pub async fn approve_action(
    Extension(_state): Extension<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<ApprovalResponse>), ProblemDetail> {
    // TODO: wire to approval queue resolve
    Err(ProblemDetail::from_status(StatusCode::NOT_FOUND)
        .with_detail(format!("Approval request not found: {id}")))
}

/// `POST /api/v1/approvals/:id/reject` — reject a pending action.
#[utoipa::path(
    post,
    path = "/api/v1/approvals/{id}/reject",
    params(("id" = String, Path, description = "Approval request identifier")),
    responses(
        (status = 200, description = "Action rejected", body = ApprovalResponse),
        (status = 404, description = "Approval request not found")
    ),
    tag = "approvals"
)]
pub async fn reject_action(
    Extension(_state): Extension<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<(StatusCode, Json<ApprovalResponse>), ProblemDetail> {
    // TODO: wire to approval queue resolve
    Err(ProblemDetail::from_status(StatusCode::NOT_FOUND)
        .with_detail(format!("Approval request not found: {id}")))
}
