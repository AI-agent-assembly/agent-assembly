//! Agent management endpoints.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::pagination::{PaginatedResponse, PaginationParams};
use crate::state::AppState;

/// JSON representation of an agent returned by the API.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgentResponse {
    /// Hex-encoded agent UUID.
    pub id: String,
    /// Human-readable agent name.
    pub name: String,
    /// Agent framework (e.g. "langgraph", "crewai").
    pub framework: String,
    /// Semver version string.
    pub version: String,
    /// Current runtime status.
    pub status: String,
    /// Tools declared at registration.
    pub tool_names: Vec<String>,
    /// Arbitrary metadata key-value pairs.
    pub metadata: BTreeMap<String, String>,
}

/// `GET /api/v1/agents` — list all registered agents with pagination.
#[utoipa::path(
    get,
    path = "/api/v1/agents",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of agents")
    ),
    tag = "agents"
)]
pub async fn list_agents(
    Extension(state): Extension<AppState>,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> impl IntoResponse {
    let all = state.agent_registry.list();
    let total = all.len() as u64;
    let offset = params.offset();
    let per_page = params.per_page();

    let items: Vec<AgentResponse> = all
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .map(|r| AgentResponse {
            id: r.agent_id.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            name: r.name,
            framework: r.framework,
            version: r.version,
            status: format!("{:?}", r.status),
            tool_names: r.tool_names,
            metadata: r.metadata,
        })
        .collect();

    (
        StatusCode::OK,
        Json(PaginatedResponse {
            items,
            page: params.page(),
            per_page,
            total,
        }),
    )
}
