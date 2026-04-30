//! HTTP client for fetching status data from the governance gateway.

use reqwest::Client;

use crate::error::CliError;

/// Client for making status-related API requests.
pub struct StatusClient {
    base_url: String,
    http: Client,
}

impl StatusClient {
    /// Create a new `StatusClient` targeting the given gateway base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::new(),
        }
    }

    /// Build a full URL for the given API path.
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Return a reference to the underlying HTTP client (for testing).
    #[cfg(test)]
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// Return the base URL (for error messages).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
