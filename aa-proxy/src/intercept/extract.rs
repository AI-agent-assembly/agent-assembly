//! Per-provider LLM request/response body extractors.
//!
//! Each extractor is a pure function that takes raw body bytes and returns
//! [`LlmFields`] with the audit-relevant fields extracted from the JSON payload.

use thiserror::Error;

/// Audit-relevant fields extracted from an LLM API request or response body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LlmFields {
    /// Model identifier (e.g. `"gpt-4"`, `"claude-3-opus-20240229"`).
    pub model: String,
    /// Number of prompt/input tokens (from request estimate or response usage).
    pub prompt_tokens: Option<u64>,
    /// Number of completion/output tokens (from response usage only).
    pub completion_tokens: Option<u64>,
    /// Number of messages in the request conversation.
    pub messages_count: u32,
}

// ── OpenAI ──────────────────────────────────────────────────────────────

/// Extract LLM fields from an OpenAI API request or response body.
///
/// Handles both request payloads (`{"model":"...","messages":[...]}`) and
/// response payloads with usage stats (`{"usage":{"prompt_tokens":...}}`).
pub fn extract_openai(body: &[u8]) -> Result<LlmFields, ExtractionError> {
    #[derive(serde::Deserialize)]
    struct OpenAiBody {
        model: Option<String>,
        messages: Option<Vec<serde_json::Value>>,
        usage: Option<OpenAiUsage>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAiUsage {
        prompt_tokens: Option<u64>,
        completion_tokens: Option<u64>,
    }

    let parsed: OpenAiBody = serde_json::from_slice(body)?;

    let model = parsed.model.unwrap_or_default();
    if model.is_empty() && parsed.messages.is_none() && parsed.usage.is_none() {
        return Err(ExtractionError::UnrecognizedFormat {
            reason: "no model, messages, or usage fields found".into(),
        });
    }

    Ok(LlmFields {
        model,
        prompt_tokens: parsed.usage.as_ref().and_then(|u| u.prompt_tokens),
        completion_tokens: parsed.usage.as_ref().and_then(|u| u.completion_tokens),
        messages_count: parsed.messages.map(|m| m.len() as u32).unwrap_or(0),
    })
}

// ── Anthropic ───────────────────────────────────────────────────────────

/// Extract LLM fields from an Anthropic API request or response body.
///
/// Anthropic uses `input_tokens`/`output_tokens` in its `usage` block
/// (unlike OpenAI's `prompt_tokens`/`completion_tokens`).
pub fn extract_anthropic(body: &[u8]) -> Result<LlmFields, ExtractionError> {
    #[derive(serde::Deserialize)]
    struct AnthropicBody {
        model: Option<String>,
        messages: Option<Vec<serde_json::Value>>,
        usage: Option<AnthropicUsage>,
    }
    #[derive(serde::Deserialize)]
    struct AnthropicUsage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    }

    let parsed: AnthropicBody = serde_json::from_slice(body)?;

    let model = parsed.model.unwrap_or_default();
    if model.is_empty() && parsed.messages.is_none() && parsed.usage.is_none() {
        return Err(ExtractionError::UnrecognizedFormat {
            reason: "no model, messages, or usage fields found".into(),
        });
    }

    Ok(LlmFields {
        model,
        prompt_tokens: parsed.usage.as_ref().and_then(|u| u.input_tokens),
        completion_tokens: parsed.usage.as_ref().and_then(|u| u.output_tokens),
        messages_count: parsed.messages.map(|m| m.len() as u32).unwrap_or(0),
    })
}

// ── Cohere ──────────────────────────────────────────────────────────────

/// Extract LLM fields from a Cohere API request or response body.
///
/// Cohere's chat endpoint uses `message` (singular string) instead of
/// `messages` (array), and reports tokens in `meta.tokens`.
pub fn extract_cohere(body: &[u8]) -> Result<LlmFields, ExtractionError> {
    #[derive(serde::Deserialize)]
    struct CohereBody {
        model: Option<String>,
        message: Option<String>,
        chat_history: Option<Vec<serde_json::Value>>,
        meta: Option<CohereMeta>,
    }
    #[derive(serde::Deserialize)]
    struct CohereMeta {
        tokens: Option<CohereTokens>,
    }
    #[derive(serde::Deserialize)]
    struct CohereTokens {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
    }

    let parsed: CohereBody = serde_json::from_slice(body)?;

    let model = parsed.model.unwrap_or_default();
    if model.is_empty() && parsed.message.is_none() && parsed.meta.is_none() {
        return Err(ExtractionError::UnrecognizedFormat {
            reason: "no model, message, or meta fields found".into(),
        });
    }

    // Count messages: 1 for the current message + chat_history length.
    let history_count = parsed.chat_history.map(|h| h.len() as u32).unwrap_or(0);
    let messages_count = if parsed.message.is_some() {
        history_count + 1
    } else {
        history_count
    };

    Ok(LlmFields {
        model,
        prompt_tokens: parsed
            .meta
            .as_ref()
            .and_then(|m| m.tokens.as_ref())
            .and_then(|t| t.input_tokens),
        completion_tokens: parsed
            .meta
            .as_ref()
            .and_then(|m| m.tokens.as_ref())
            .and_then(|t| t.output_tokens),
        messages_count,
    })
}

/// Errors that can occur during body extraction.
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// The body is not valid JSON.
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// The JSON is valid but does not match the expected provider schema.
    #[error("unrecognized format: {reason}")]
    UnrecognizedFormat { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OpenAI tests ────────────────────────────────────────────────────

    #[test]
    fn openai_minimal_request() {
        let body = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
        let fields = extract_openai(body).unwrap();
        assert_eq!(fields.model, "gpt-4");
        assert_eq!(fields.messages_count, 1);
        assert_eq!(fields.prompt_tokens, None);
        assert_eq!(fields.completion_tokens, None);
    }

    #[test]
    fn openai_response_with_usage() {
        let body = br#"{
            "model": "gpt-4",
            "choices": [],
            "usage": {"prompt_tokens": 10, "completion_tokens": 20}
        }"#;
        let fields = extract_openai(body).unwrap();
        assert_eq!(fields.model, "gpt-4");
        assert_eq!(fields.prompt_tokens, Some(10));
        assert_eq!(fields.completion_tokens, Some(20));
    }

    #[test]
    fn openai_malformed_json_returns_error() {
        let body = b"not json";
        let err = extract_openai(body).unwrap_err();
        assert!(matches!(err, ExtractionError::InvalidJson(_)));
    }

    #[test]
    fn openai_empty_object_returns_unrecognized() {
        let body = br#"{}"#;
        let err = extract_openai(body).unwrap_err();
        assert!(matches!(err, ExtractionError::UnrecognizedFormat { .. }));
    }

    // ── Anthropic tests ─────────────────────────────────────────────────

    #[test]
    fn anthropic_minimal_request() {
        let body = br#"{"model":"claude-3-opus-20240229","messages":[{"role":"user","content":"hi"}]}"#;
        let fields = extract_anthropic(body).unwrap();
        assert_eq!(fields.model, "claude-3-opus-20240229");
        assert_eq!(fields.messages_count, 1);
        assert_eq!(fields.prompt_tokens, None);
    }

    #[test]
    fn anthropic_response_with_usage() {
        let body = br#"{
            "model": "claude-3-opus-20240229",
            "content": [],
            "usage": {"input_tokens": 15, "output_tokens": 30}
        }"#;
        let fields = extract_anthropic(body).unwrap();
        assert_eq!(fields.model, "claude-3-opus-20240229");
        assert_eq!(fields.prompt_tokens, Some(15));
        assert_eq!(fields.completion_tokens, Some(30));
    }

    #[test]
    fn anthropic_malformed_json_returns_error() {
        let body = b"{invalid";
        let err = extract_anthropic(body).unwrap_err();
        assert!(matches!(err, ExtractionError::InvalidJson(_)));
    }

    #[test]
    fn anthropic_empty_object_returns_unrecognized() {
        let body = br#"{}"#;
        let err = extract_anthropic(body).unwrap_err();
        assert!(matches!(err, ExtractionError::UnrecognizedFormat { .. }));
    }

    // ── Cohere tests ────────────────────────────────────────────────────

    #[test]
    fn cohere_minimal_request() {
        let body = br#"{"model":"command-r-plus","message":"hello"}"#;
        let fields = extract_cohere(body).unwrap();
        assert_eq!(fields.model, "command-r-plus");
        assert_eq!(fields.messages_count, 1);
        assert_eq!(fields.prompt_tokens, None);
    }

    #[test]
    fn cohere_response_with_meta_tokens() {
        let body = br#"{
            "model": "command-r-plus",
            "text": "response",
            "meta": {"tokens": {"input_tokens": 5, "output_tokens": 12}}
        }"#;
        let fields = extract_cohere(body).unwrap();
        assert_eq!(fields.model, "command-r-plus");
        assert_eq!(fields.prompt_tokens, Some(5));
        assert_eq!(fields.completion_tokens, Some(12));
    }

    #[test]
    fn cohere_with_chat_history() {
        let body = br#"{
            "model": "command-r",
            "message": "next question",
            "chat_history": [
                {"role": "USER", "message": "first"},
                {"role": "CHATBOT", "message": "reply"}
            ]
        }"#;
        let fields = extract_cohere(body).unwrap();
        assert_eq!(fields.messages_count, 3); // 2 history + 1 current
    }

    #[test]
    fn cohere_malformed_json_returns_error() {
        let body = b"<<<";
        let err = extract_cohere(body).unwrap_err();
        assert!(matches!(err, ExtractionError::InvalidJson(_)));
    }

    #[test]
    fn cohere_empty_object_returns_unrecognized() {
        let body = br#"{}"#;
        let err = extract_cohere(body).unwrap_err();
        assert!(matches!(err, ExtractionError::UnrecognizedFormat { .. }));
    }
}
