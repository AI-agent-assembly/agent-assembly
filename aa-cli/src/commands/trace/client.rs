//! HTTP client for fetching session traces from the gateway API.

use crate::config::ResolvedContext;

/// Build the full URL for the trace endpoint.
pub fn build_trace_url(ctx: &ResolvedContext, session_id: &str) -> String {
    format!(
        "{}/api/v1/traces/{}",
        ctx.api_url.trim_end_matches('/'),
        session_id
    )
}
