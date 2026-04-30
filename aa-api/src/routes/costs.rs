//! Cost and budget summary endpoints.

use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// JSON representation of the cost/budget summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CostSummary {
    /// Total spend today in USD.
    pub daily_spend_usd: String,
    /// Total spend this month in USD (if monthly tracking is enabled).
    pub monthly_spend_usd: Option<String>,
    /// Calendar date (YYYY-MM-DD) the daily spend applies to.
    pub date: String,
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

    let summary = CostSummary {
        daily_spend_usd: snapshot.global.spent_usd.to_string(),
        monthly_spend_usd: snapshot.global.monthly_spent_usd.map(|d| d.to_string()),
        date: snapshot.global.date.to_string(),
    };

    (StatusCode::OK, Json(summary))
}
