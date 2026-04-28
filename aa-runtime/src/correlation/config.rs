//! Configuration for the causal correlation engine.

/// Configuration parameters for the [`super::CorrelationEngine`].
#[derive(Debug, Clone)]
pub struct CorrelationConfig {
    /// Maximum time window (in milliseconds) within which an intent and an
    /// action must occur to be considered causally correlated.
    ///
    /// Default: 5000 ms.
    pub window_ms: u64,
    /// Maximum number of events held in the sliding window before the oldest
    /// events are force-evicted regardless of age.
    pub max_window_size: usize,
    /// How often (in milliseconds) the engine runs TTL eviction on the sliding
    /// window to discard events older than `window_ms`.
    pub eviction_interval_ms: u64,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            window_ms: 5_000,
            max_window_size: 10_000,
            eviction_interval_ms: 1_000,
        }
    }
}
