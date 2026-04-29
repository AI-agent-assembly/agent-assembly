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
