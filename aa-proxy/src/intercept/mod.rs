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

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::intercept::detect::LlmApiPattern;
    use crate::intercept::event::ProxyEvent;

    fn make_event(pattern: LlmApiPattern) -> ProxyEvent {
        ProxyEvent {
            agent_id: Some("test-agent".into()),
            pattern,
            method: "POST".into(),
            path: "/v1/chat/completions".into(),
            request_body: None,
            response_body: None,
            timestamp: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn intercept_openai_event_succeeds() {
        let interceptor = Interceptor::new();
        let result = interceptor.intercept(make_event(LlmApiPattern::OpenAi)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_anthropic_event_succeeds() {
        let interceptor = Interceptor::new();
        let result = interceptor.intercept(make_event(LlmApiPattern::Anthropic)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_unknown_event_succeeds() {
        let interceptor = Interceptor::new();
        let result = interceptor.intercept(make_event(LlmApiPattern::Unknown)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_with_no_agent_id_succeeds() {
        let interceptor = Interceptor::new();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.agent_id = None;
        assert!(interceptor.intercept(event).await.is_ok());
    }
}
