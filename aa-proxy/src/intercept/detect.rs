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
///
/// Comparison is case-insensitive. A host like `api.openai.com:443` is
/// normalised by stripping the port before matching.
pub fn detect_api(host: &str) -> LlmApiPattern {
    let hostname = host.split(':').next().unwrap_or(host);
    match hostname.to_ascii_lowercase().as_str() {
        "api.openai.com" => LlmApiPattern::OpenAi,
        "api.anthropic.com" => LlmApiPattern::Anthropic,
        "api.cohere.com" => LlmApiPattern::Cohere,
        _ => LlmApiPattern::Unknown,
    }
}
