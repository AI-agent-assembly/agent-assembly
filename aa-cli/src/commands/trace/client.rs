//! HTTP client for fetching session traces from the gateway API.

use crate::config::ResolvedContext;
use crate::error::CliError;

use super::models::SessionTrace;

/// Build the full URL for the trace endpoint.
pub fn build_trace_url(ctx: &ResolvedContext, session_id: &str) -> String {
    format!(
        "{}/api/v1/traces/{}",
        ctx.api_url.trim_end_matches('/'),
        session_id
    )
}

/// Fetch a session trace from the gateway API.
pub async fn fetch_trace(
    ctx: &ResolvedContext,
    session_id: &str,
) -> Result<SessionTrace, CliError> {
    let url = build_trace_url(ctx, session_id);
    let client = reqwest::Client::new();

    let mut request = client.get(&url);
    if let Some(ref key) = ctx.api_key {
        request = request.bearer_auth(key);
    }

    let response = request.send().await?.error_for_status()?;
    let trace: SessionTrace = response.json().await?;
    Ok(trace)
}
