//! Cost and budget summary endpoints.

use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// Per-agent cost entry within the budget summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgentCostEntry {
    /// Agent identifier (hex-encoded).
    pub agent_id: String,
    /// Daily spend for this agent in USD.
    pub daily_spend_usd: String,
}

/// JSON representation of the cost/budget summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CostSummary {
    /// Total spend today in USD.
    pub daily_spend_usd: String,
    /// Total spend this month in USD (if monthly tracking is enabled).
    pub monthly_spend_usd: Option<String>,
    /// Calendar date (YYYY-MM-DD) the daily spend applies to.
    pub date: String,
    /// Configured daily budget limit in USD, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_limit_usd: Option<String>,
    /// Configured monthly budget limit in USD, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_limit_usd: Option<String>,
    /// Per-agent cost breakdown for the current day.
    #[serde(default)]
    pub per_agent: Vec<AgentCostEntry>,
}

/// `GET /api/v1/costs` — cost and budget summary.
///
/// Retrieve the current daily and monthly cost and budget summary.
#[utoipa::path(
    get,
    path = "/api/v1/costs",
    responses(
        (status = 200, description = "Cost and budget summary", body = CostSummary)
    ),
    tag = "costs"
)]
pub async fn get_cost_summary(Extension(state): Extension<AppState>) -> (StatusCode, Json<CostSummary>) {
    let snapshot = state.budget_tracker.snapshot();

    let per_agent: Vec<AgentCostEntry> = snapshot
        .per_agent
        .iter()
        .map(|entry| AgentCostEntry {
            agent_id: entry.agent_id_hex.clone(),
            daily_spend_usd: entry.state.spent_usd.to_string(),
        })
        .collect();

    let summary = CostSummary {
        daily_spend_usd: snapshot.global.spent_usd.to_string(),
        monthly_spend_usd: snapshot.global.monthly_spent_usd.map(|d| d.to_string()),
        date: snapshot.global.date.to_string(),
        daily_limit_usd: state.budget_tracker.daily_limit_usd().map(|d| d.to_string()),
        monthly_limit_usd: state.budget_tracker.monthly_limit_usd().map(|d| d.to_string()),
        per_agent,
    };

    (StatusCode::OK, Json(summary))
}
