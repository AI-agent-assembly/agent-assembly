//! Policy management endpoints.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ProblemDetail;
use crate::pagination::{PaginatedResponse, PaginationParams};
use crate::state::AppState;

/// JSON representation of a governance policy version.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PolicyResponse {
    /// Policy name from metadata.
    pub name: String,
    /// Policy version string.
    pub version: String,
    /// Whether this is the currently active policy.
    pub active: bool,
    /// Number of rules in this policy version.
    pub rule_count: usize,
}

/// `GET /api/v1/policies` — list all policy versions.
///
/// List all governance policy versions with pagination.
#[utoipa::path(
    get,
    path = "/api/v1/policies",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of policy versions", body = Vec<PolicyResponse>)
    ),
    tag = "policies"
)]
pub async fn list_policies(
    Extension(_state): Extension<AppState>,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> impl IntoResponse {
    // TODO: wire to policy version store once available
    let items: Vec<PolicyResponse> = Vec::new();

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

/// Request body for creating a new policy.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePolicyRequest {
    /// Raw YAML content of the governance policy.
    pub policy_yaml: String,
}

/// `POST /api/v1/policies` — apply a new governance policy.
///
/// Submit and activate a new governance policy from YAML.
#[utoipa::path(
    post,
    path = "/api/v1/policies",
    request_body = CreatePolicyRequest,
    responses(
        (status = 201, description = "Policy created", body = PolicyResponse),
        (status = 400, description = "Invalid policy YAML")
    ),
    tag = "policies"
)]
pub async fn create_policy(
    Extension(_state): Extension<AppState>,
    Json(_body): Json<CreatePolicyRequest>,
) -> Result<(StatusCode, Json<PolicyResponse>), ProblemDetail> {
    // TODO: validate YAML, store version, reload engine
    Err(ProblemDetail::from_status(StatusCode::NOT_IMPLEMENTED).with_detail("Policy creation not yet wired"))
}

/// `GET /api/v1/policies/active` — get the currently active policy.
///
/// Retrieve the currently active governance policy.
#[utoipa::path(
    get,
    path = "/api/v1/policies/active",
    responses(
        (status = 200, description = "Currently active policy", body = PolicyResponse),
        (status = 404, description = "No active policy loaded")
    ),
    tag = "policies"
)]
pub async fn get_active_policy(
    Extension(_state): Extension<AppState>,
) -> Result<(StatusCode, Json<PolicyResponse>), ProblemDetail> {
    // TODO: read active policy from engine
    Err(ProblemDetail::from_status(StatusCode::NOT_FOUND).with_detail("No active policy loaded"))
}
