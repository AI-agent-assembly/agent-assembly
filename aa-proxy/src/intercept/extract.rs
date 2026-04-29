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
