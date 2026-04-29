//! HTTP webhook delivery target.
//!
//! [`WebhookTarget`] wraps a shared [`reqwest::Client`] and a destination URL.
//! It serializes event envelopes as JSON and POSTs them with fire-and-forget
//! semantics — failures are logged via `tracing::warn!` but never block the
//! event pipeline.

use reqwest::Client;
use serde_json::Value;

use super::PublishError;

/// Delivers JSON event envelopes to a single webhook URL via HTTP POST.
///
/// Uses a shared [`Client`] for connection pooling across deliveries.
pub struct WebhookTarget {
    client: Client,
    url: String,
}

impl WebhookTarget {
    /// Create a new webhook target for the given URL.
    ///
    /// The [`Client`] is shared across all deliveries for connection reuse.
    pub fn new(client: Client, url: String) -> Self {
        Self { client, url }
    }

    /// POST the JSON envelope to the webhook URL.
    ///
    /// Returns `Ok(())` on 2xx, logs and returns `Err` otherwise.
    pub async fn deliver(&self, envelope: &Value) -> Result<(), PublishError> {
        let response = self
            .client
            .post(&self.url)
            .json(envelope)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(
                status = %status,
                url = %self.url,
                body = %body,
                "webhook delivery received non-2xx response"
            );
            return Err(PublishError::Serialization(format!(
                "webhook returned {status}"
            )));
        }

        tracing::debug!(url = %self.url, "webhook delivery succeeded");
        Ok(())
    }

    /// Returns the configured webhook URL.
    pub fn url(&self) -> &str {
        &self.url
    }
}
