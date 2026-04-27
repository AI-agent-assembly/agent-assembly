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
    /// construct and return the corresponding [`event::ProxyEvent`].
    pub async fn intercept(&self, _event: event::ProxyEvent) -> Result<(), ProxyError> {
        todo!()
    }
}

impl Default for Interceptor {
    fn default() -> Self {
        Self::new()
    }
}
