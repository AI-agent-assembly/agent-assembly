//! Traffic interception: detect LLM API calls and emit structured events.

pub mod detect;
pub mod event;
pub mod extract;

use tokio::sync::broadcast;

use aa_runtime::pipeline::PipelineEvent;

use crate::error::ProxyError;
use crate::intercept::detect::LlmApiPattern;
use crate::intercept::extract::{extract_anthropic, extract_cohere, extract_openai, ExtractionError, LlmFields};

/// Inspects a decrypted HTTP request/response pair, decides whether it is an
/// LLM API call, and extracts audit-relevant fields from the body.
///
/// Holds a [`broadcast::Sender`] to emit [`PipelineEvent`]s for intercepted
/// LLM calls into the runtime event pipeline.
pub struct Interceptor {
    event_tx: broadcast::Sender<PipelineEvent>,
}

impl Interceptor {
    /// Create a new `Interceptor` that emits events on the given broadcast channel.
    pub fn new(event_tx: broadcast::Sender<PipelineEvent>) -> Self {
        Self { event_tx }
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

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use bytes::Bytes;

    use super::*;
    use crate::intercept::detect::LlmApiPattern;
    use crate::intercept::event::ProxyEvent;

    /// Create a dummy `Interceptor` with a broadcast sender whose receiver is
    /// dropped — sends silently fail, which is correct for unit tests that
    /// only verify extraction logic.
    fn make_interceptor() -> Interceptor {
        let (tx, _rx) = broadcast::channel(16);
        Interceptor::new(tx)
    }

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
        let interceptor = make_interceptor();
        let result = interceptor.intercept(&make_event(LlmApiPattern::OpenAi)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_anthropic_event_succeeds() {
        let interceptor = make_interceptor();
        let result = interceptor.intercept(&make_event(LlmApiPattern::Anthropic)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn intercept_unknown_returns_none() {
        let interceptor = make_interceptor();
        let result = interceptor
            .intercept(&make_event(LlmApiPattern::Unknown))
            .await
            .unwrap();
        assert!(result.is_none(), "unknown pattern should skip extraction");
    }

    #[tokio::test]
    async fn intercept_with_no_agent_id_succeeds() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.agent_id = None;
        assert!(interceptor.intercept(&event).await.is_ok());
    }

    #[tokio::test]
    async fn intercept_openai_with_body_extracts_fields() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.response_body = Some(Bytes::from(
            r#"{"model":"gpt-4","usage":{"prompt_tokens":10,"completion_tokens":20}}"#,
        ));
        let fields = interceptor.intercept(&event).await.unwrap().unwrap();
        assert_eq!(fields.model, "gpt-4");
        assert_eq!(fields.prompt_tokens, Some(10));
        assert_eq!(fields.completion_tokens, Some(20));
    }

    #[tokio::test]
    async fn intercept_anthropic_with_body_extracts_fields() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::Anthropic);
        event.response_body = Some(Bytes::from(
            r#"{"model":"claude-3-opus-20240229","usage":{"input_tokens":15,"output_tokens":30}}"#,
        ));
        let fields = interceptor.intercept(&event).await.unwrap().unwrap();
        assert_eq!(fields.model, "claude-3-opus-20240229");
        assert_eq!(fields.prompt_tokens, Some(15));
        assert_eq!(fields.completion_tokens, Some(30));
    }

    #[tokio::test]
    async fn intercept_cohere_with_body_extracts_fields() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::Cohere);
        event.response_body = Some(Bytes::from(
            r#"{"model":"command-r-plus","message":"hello","meta":{"tokens":{"input_tokens":5,"output_tokens":12}}}"#,
        ));
        let fields = interceptor.intercept(&event).await.unwrap().unwrap();
        assert_eq!(fields.model, "command-r-plus");
        assert_eq!(fields.prompt_tokens, Some(5));
        assert_eq!(fields.completion_tokens, Some(12));
        assert_eq!(fields.messages_count, 1);
    }

    #[tokio::test]
    async fn intercept_prefers_response_body_over_request() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.request_body = Some(Bytes::from(
            r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#,
        ));
        event.response_body = Some(Bytes::from(
            r#"{"model":"gpt-4","usage":{"prompt_tokens":10,"completion_tokens":20}}"#,
        ));
        let fields = interceptor.intercept(&event).await.unwrap().unwrap();
        // Response body was used — it has usage stats, not messages
        assert_eq!(fields.prompt_tokens, Some(10));
        assert_eq!(fields.completion_tokens, Some(20));
        assert_eq!(fields.messages_count, 0);
    }

    #[tokio::test]
    async fn intercept_falls_back_to_request_body() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.request_body = Some(Bytes::from(
            r#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#,
        ));
        event.response_body = None;
        let fields = interceptor.intercept(&event).await.unwrap().unwrap();
        assert_eq!(fields.model, "gpt-4");
        assert_eq!(fields.messages_count, 1);
        assert_eq!(fields.prompt_tokens, None);
    }

    #[tokio::test]
    async fn intercept_with_none_body_returns_none() {
        let interceptor = make_interceptor();
        let event = make_event(LlmApiPattern::OpenAi);
        // Both request_body and response_body are None
        let result = interceptor.intercept(&event).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn intercept_with_malformed_body_returns_none() {
        let interceptor = make_interceptor();
        let mut event = make_event(LlmApiPattern::OpenAi);
        event.response_body = Some(Bytes::from("not json"));
        // Malformed body logs a warning and returns None (not an error)
        let result = interceptor.intercept(&event).await.unwrap();
        assert!(result.is_none());
    }
}
