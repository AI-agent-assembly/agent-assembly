//! HTTP client for cost API queries.

use crate::config::ResolvedContext;
use crate::error::CliError;

use super::models::CostResponse;

/// Fetch cost summary from the gateway API.
pub async fn fetch_costs(ctx: &ResolvedContext) -> Result<CostResponse, CliError> {
    crate::client::get_json(ctx, "/api/v1/costs").await
}
