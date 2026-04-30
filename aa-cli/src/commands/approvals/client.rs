//! HTTP client functions for the `aasm approvals` subcommand.

/// Build the base URL for the approvals API endpoint.
///
/// Strips trailing slashes from the base URL and appends
/// `/api/v1/approvals`.
pub fn build_approvals_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    format!("{base}/api/v1/approvals")
}
