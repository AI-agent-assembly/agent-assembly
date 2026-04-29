//! Traffic interception: detect LLM API calls and emit structured events.

pub mod detect;
pub mod event;
pub mod extract;

use crate::error::ProxyError;
use crate::intercept::detect::LlmApiPattern;
use crate::intercept::extract::{extract_anthropic, extract_cohere, extract_openai, ExtractionError, LlmFields};

/// Inspects a decrypted HTTP request/response pair, decides whether it is an
/// LLM API call, and extracts audit-relevant fields from the body.
pub struct Interceptor;

impl Interceptor {
    /// Create a new `Interceptor`.
    pub fn new() -> Self {
        Self
    }

    /// Inspect an intercepted exchange, extract LLM fields from the body
    /// (if available), and log the result.
    ///
    /// Full policy evaluation (forwarding to `aa-gateway`) and pipeline
    /// integration (`broadcast::Sender<PipelineEvent>`) will be added in a
    /// future ticket. For now this extracts and logs.
    pub async fn intercept(&self, event: &event::ProxyEvent) -> Result<Option<LlmFields>, ProxyError> {
        // Non-LLM traffic is passed through without extraction.
        if event.pattern == LlmApiPattern::Unknown {
            tracing::debug!(method = %event.method, path = %event.path, "non-LLM traffic, skipping");
            return Ok(None);
        }

        // Pick the body to extract from: prefer response (has usage stats),
        // fall back to request.
        let body = event.response_body.as_ref().or(event.request_body.as_ref());

        let fields = match body {
            Some(bytes) => match Self::extract_for_pattern(&event.pattern, bytes) {
                Ok(f) => Some(f),
                Err(e) => {
                    tracing::warn!(
                        pattern = ?event.pattern,
                        error = %e,
                        "failed to extract LLM fields from body"
                    );
                    None
                }
            },
            None => None,
        };

        tracing::info!(
            agent_id = event.agent_id.as_deref().unwrap_or("<unknown>"),
            pattern = ?event.pattern,
            method = %event.method,
            path = %event.path,
            model = fields.as_ref().map(|f| f.model.as_str()).unwrap_or("<unknown>"),
            messages = fields.as_ref().map(|f| f.messages_count).unwrap_or(0),
            "intercepted LLM API call"
        );

        Ok(fields)
    }

    /// Select the correct extractor based on the detected API pattern.
    fn extract_for_pattern(pattern: &LlmApiPattern, body: &[u8]) -> Result<LlmFields, ExtractionError> {
        match pattern {
            LlmApiPattern::OpenAi => extract_openai(body),
            LlmApiPattern::Anthropic => extract_anthropic(body),
            LlmApiPattern::Cohere => extract_cohere(body),
            LlmApiPattern::Unknown => Err(ExtractionError::UnrecognizedFormat {
                reason: "unknown provider".into(),
            }),
        }
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
        let result = interceptor.intercept(&make_event(LlmApiPattern::OpenAi)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_anthropic_event_succeeds() {
        let interceptor = Interceptor::new();
        let result = interceptor.intercept(&make_event(LlmApiPattern::Anthropic)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_unknown_returns_none() {
        let interceptor = Interceptor::new();
        let result = interceptor
            .intercept(&make_event(LlmApiPattern::Unknown))
            .await
            .unwrap();
        assert!(result.is_none(), "unknown pattern should skip extraction");
    }

    #[tokio::test]
    async fn intercept_with_no_agent_id_succeeds() {
        let interceptor = Interceptor::new();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.agent_id = None;
        assert!(interceptor.intercept(&event).await.is_ok());
    }
}
