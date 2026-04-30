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
    Extension(state): Extension<AppState>,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let all = state
        .policy_history
        .list(usize::MAX)
        .await
        .map_err(|e| ProblemDetail::from_status(StatusCode::INTERNAL_SERVER_ERROR).with_detail(format!("{e:?}")))?;

    let total = all.len() as u64;

    let items: Vec<PolicyResponse> = all
        .into_iter()
        .skip(params.offset())
        .take(params.per_page() as usize)
        .enumerate()
        .map(|(i, meta)| PolicyResponse {
            name: meta.sha256[..12].to_string(),
            version: meta.timestamp,
            active: i == 0 && params.page() == 1,
            rule_count: 0,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(PaginatedResponse {
            items,
            page: params.page(),
            per_page: params.per_page(),
            total,
        }),
    ))
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
    Extension(state): Extension<AppState>,
    Json(body): Json<CreatePolicyRequest>,
) -> Result<(StatusCode, Json<PolicyResponse>), ProblemDetail> {
    let meta = state
        .policy_engine
        .apply_yaml(&body.policy_yaml, Some("api"), state.policy_history.as_ref())
        .await
        .map_err(|e| {
            ProblemDetail::from_status(StatusCode::BAD_REQUEST).with_detail(format!("Invalid policy: {e:?}"))
        })?;

    Ok((
        StatusCode::CREATED,
        Json(PolicyResponse {
            name: meta.sha256[..12].to_string(),
            version: meta.timestamp,
            active: true,
            rule_count: 0,
        }),
    ))
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
    Extension(state): Extension<AppState>,
) -> Result<(StatusCode, Json<PolicyResponse>), ProblemDetail> {
    let info = state.policy_engine.active_policy_info();
    Ok((
        StatusCode::OK,
        Json(PolicyResponse {
            name: info.name.unwrap_or_else(|| "unnamed".to_string()),
            version: info.policy_version.unwrap_or_else(|| "unknown".to_string()),
            active: true,
            rule_count: info.rule_count,
        }),
    ))
}
