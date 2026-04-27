//! In-process pipeline metrics counters.
//!
//! Counters are stored as `AtomicU64` values so they can be updated from
//! the pipeline task and read from health/metrics endpoints without locking.
//! The HTTP exposure of these counters is handled by AAASM-32.

use std::sync::atomic::{AtomicU64, Ordering};

/// Shared metrics for the event aggregation pipeline.
///
/// Wrap in an `Arc` and pass clones to both the pipeline task and any
/// downstream consumers that need to record dropped events.
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    events_processed_total: AtomicU64,
    events_dropped_total:   AtomicU64,
    last_batch_size:        AtomicU64,
}

impl PipelineMetrics {
    /// Increment the processed-events counter by `n`.
    pub fn record_processed(&self, n: u64) {
        self.events_processed_total.fetch_add(n, Ordering::Relaxed);
    }

    /// Increment the dropped-events counter by `n`.
    ///
    /// Called by broadcast subscribers when they receive
    /// `RecvError::Lagged(n)`.
    pub fn record_dropped(&self, n: u64) {
        self.events_dropped_total.fetch_add(n, Ordering::Relaxed);
    }

    /// Record the size of the most recently flushed batch.
    ///
    /// This is a last-value gauge, not a cumulative counter — each flush
    /// overwrites the previous value.
    pub fn record_batch_size(&self, n: u64) {
        self.last_batch_size.store(n, Ordering::Relaxed);
    }

    /// Read the current total processed-events count.
    pub fn processed(&self) -> u64 {
        self.events_processed_total.load(Ordering::Relaxed)
    }

    /// Read the current total dropped-events count.
    pub fn dropped(&self) -> u64 {
        self.events_dropped_total.load(Ordering::Relaxed)
    }

    /// Read the size of the most recently flushed batch.
    pub fn last_batch_size(&self) -> u64 {
        self.last_batch_size.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_all_zeros() {
        let metrics = PipelineMetrics::default();
        assert_eq!(metrics.processed(), 0);
        assert_eq!(metrics.dropped(), 0);
        assert_eq!(metrics.last_batch_size(), 0);
    }

    #[test]
    fn record_processed_accumulates() {
        let metrics = PipelineMetrics::default();
        metrics.record_processed(5);
        metrics.record_processed(3);
        assert_eq!(metrics.processed(), 8);
    }

    #[test]
    fn record_dropped_accumulates() {
        let metrics = PipelineMetrics::default();
        metrics.record_dropped(10);
        metrics.record_dropped(2);
        assert_eq!(metrics.dropped(), 12);
    }

    #[test]
    fn record_batch_size_is_last_value() {
        let metrics = PipelineMetrics::default();
        metrics.record_batch_size(100);
        metrics.record_batch_size(42);
        assert_eq!(metrics.last_batch_size(), 42);
    }

    #[test]
    fn metrics_are_independent() {
        let metrics = PipelineMetrics::default();
        metrics.record_processed(7);
        metrics.record_dropped(3);
        assert_eq!(metrics.processed(), 7);
        assert_eq!(metrics.dropped(), 3);
    }
}
