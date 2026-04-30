//! HTTP client functions for the `aasm approvals` subcommand.

use crate::config::ResolvedContext;
use crate::error::CliError;

use super::models::{ApprovalResponse, PaginatedResponse};

/// Build the base URL for the approvals API endpoint.
///
/// Strips trailing slashes from the base URL and appends
/// `/api/v1/approvals`.
pub fn build_approvals_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    format!("{base}/api/v1/approvals")
}

/// Fetch all pending approval requests from the API.
pub async fn list_approvals(
    ctx: &ResolvedContext,
) -> Result<PaginatedResponse<ApprovalResponse>, CliError> {
    let url = build_approvals_url(&ctx.api_url);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await?.error_for_status()?;
    let body = resp.json::<PaginatedResponse<ApprovalResponse>>().await?;
    Ok(body)
}
