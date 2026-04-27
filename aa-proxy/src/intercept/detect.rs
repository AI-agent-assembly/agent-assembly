//! LLM API pattern detection from HTTPS request host headers.
//!
//! The proxy only intercepts traffic destined for known LLM providers when
//! `ProxyConfig::llm_only` is `true`. This module provides the detection logic.

/// Identifies which LLM provider an intercepted request is targeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmApiPattern {
    /// `api.openai.com`
    OpenAi,
    /// `api.anthropic.com`
    Anthropic,
    /// `api.cohere.com`
    Cohere,
    /// Host does not match any known LLM API.
    Unknown,
}

/// Classify `host` (the CONNECT tunnel target hostname) as an [`LlmApiPattern`].
pub fn detect_api(_host: &str) -> LlmApiPattern {
    todo!()
}
