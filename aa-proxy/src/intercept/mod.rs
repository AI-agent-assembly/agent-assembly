//! Traffic interception: detect LLM API calls and emit structured events.

pub mod detect;
pub mod event;

use crate::error::ProxyError;

/// Inspects a decrypted HTTP request/response pair, decides whether it is an
/// LLM API call, and emits a [`event::ProxyEvent`] if so.
pub struct Interceptor;

impl Interceptor {
    /// Create a new `Interceptor`.
    pub fn new() -> Self {
        Self
    }

    /// Inspect an intercepted exchange and, if it matches an LLM API pattern,
    /// log and return the corresponding [`event::ProxyEvent`].
    ///
    /// Full policy evaluation (forwarding to `aa-gateway`) will be added in a
    /// future ticket. For now this captures and logs the event.
    pub async fn intercept(&self, event: event::ProxyEvent) -> Result<(), ProxyError> {
        tracing::info!(
            agent_id = event.agent_id.as_deref().unwrap_or("<unknown>"),
            pattern = ?event.pattern,
            method = %event.method,
            path = %event.path,
            "intercepted LLM API call"
        );
        Ok(())
    }
}

impl Default for Interceptor {
    fn default() -> Self {
        Self::new()
    }
}
