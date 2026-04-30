//! HTTP client functions for the `aasm approvals` subcommand.

use url::Url;

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

/// Fetch a single pending approval request by ID.
pub async fn get_approval(
    ctx: &ResolvedContext,
    id: &str,
) -> Result<ApprovalResponse, CliError> {
    let url = format!("{}/{id}", build_approvals_url(&ctx.api_url));
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await?.error_for_status()?;
    let body = resp.json::<ApprovalResponse>().await?;
    Ok(body)
}

/// Approve a pending approval request by ID.
pub async fn approve_action(
    ctx: &ResolvedContext,
    id: &str,
    reason: Option<&str>,
) -> Result<ApprovalResponse, CliError> {
    let url = format!("{}/{id}/approve", build_approvals_url(&ctx.api_url));
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "by": "cli",
        "reason": reason,
    });
    let mut req = client.post(&url).json(&body);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await?.error_for_status()?;
    let result = resp.json::<ApprovalResponse>().await?;
    Ok(result)
}

/// Reject a pending approval request by ID.
pub async fn reject_action(
    ctx: &ResolvedContext,
    id: &str,
    reason: &str,
) -> Result<ApprovalResponse, CliError> {
    let url = format!("{}/{id}/reject", build_approvals_url(&ctx.api_url));
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "by": "cli",
        "reason": reason,
    });
    let mut req = client.post(&url).json(&body);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await?.error_for_status()?;
    let result = resp.json::<ApprovalResponse>().await?;
    Ok(result)
}

/// Convert an HTTP(S) base URL to a WebSocket URL for the events endpoint.
///
/// `http://` becomes `ws://`, `https://` becomes `wss://`.
/// Appends `/api/v1/events?types={types}`.
pub fn build_ws_url(base: &str, types: &str) -> Result<String, CliError> {
    let mut parsed = Url::parse(base).map_err(|e| {
        CliError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    })?;
    let new_scheme = match parsed.scheme() {
        "https" => "wss",
        _ => "ws",
    };
    parsed
        .set_scheme(new_scheme)
        .map_err(|()| CliError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to set scheme")))?;
    let base_str = parsed.as_str().trim_end_matches('/');
    Ok(format!("{base_str}/api/v1/events?types={types}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_approvals_url_without_trailing_slash() {
        assert_eq!(
            build_approvals_url("http://localhost:8080"),
            "http://localhost:8080/api/v1/approvals"
        );
    }

    #[test]
    fn build_approvals_url_with_trailing_slash() {
        assert_eq!(
            build_approvals_url("http://localhost:8080/"),
            "http://localhost:8080/api/v1/approvals"
        );
    }
}
