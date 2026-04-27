//! Structured representation of a single intercepted LLM API call.

use std::time::SystemTime;

use bytes::Bytes;

use crate::intercept::detect::LlmApiPattern;

/// One captured LLM API call — produced by the [`super::Interceptor`] and
/// forwarded to `aa-gateway` for policy evaluation.
pub struct ProxyEvent {
    /// The agent identifier, if available from a request header.
    pub agent_id: Option<String>,

    /// Which LLM provider was called.
    pub pattern: LlmApiPattern,

    /// HTTP method (e.g. `"POST"`).
    pub method: String,

    /// Request path (e.g. `"/v1/chat/completions"`).
    pub path: String,

    /// Buffered request body, if the payload was consumed during interception.
    pub request_body: Option<Bytes>,

    /// Buffered response body, if the payload was consumed during interception.
    pub response_body: Option<Bytes>,

    /// Wall-clock time at which the request was intercepted.
    pub timestamp: SystemTime,
}
