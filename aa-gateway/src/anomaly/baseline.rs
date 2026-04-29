//! Per-agent behavioral baseline for anomaly detection.
//!
//! Maintains a sliding window of action timestamps, tool usage frequency, and
//! credential finding counts for each agent. The baseline is used by the
//! detector to identify deviations from normal behavior.
//!
//! Design note: follows the same BTreeMap-based sliding window pattern as
//! `aa-runtime::correlation::window::SlidingWindow`, but tracks action rates
//! rather than correlation events.

use std::collections::HashMap;

/// Number of time buckets used to compute mean/stddev of action rates.
/// The baseline window is divided into this many equal-sized buckets.
const RATE_BUCKETS: u64 = 12;

/// Per-agent behavioral baseline with sliding window tracking.
pub struct AgentBaseline {
    /// Timestamps (milliseconds) of all actions within the window, kept sorted.
    action_timestamps: Vec<u64>,
    /// Count of tool calls keyed by a hash of `(tool_name, args)`.
    tool_call_counts: HashMap<u64, u32>,
    /// Accumulated credential findings within the current window.
    credential_findings_count: u32,
    /// Window duration in milliseconds.
    window_ms: u64,
}

impl AgentBaseline {
    /// Create an empty baseline with the given window duration.
    pub fn new(window_secs: u64) -> Self {
        Self {
            action_timestamps: Vec::new(),
            tool_call_counts: HashMap::new(),
            credential_findings_count: 0,
            window_ms: window_secs * 1000,
        }
    }

    /// Record an action at the given timestamp and evict stale entries.
    pub fn record_action(&mut self, now_ms: u64) {
        self.evict(now_ms);
        self.action_timestamps.push(now_ms);
    }

    /// Record a tool call with the given hash and evict stale tool entries.
    pub fn record_tool_call(&mut self, tool_hash: u64, now_ms: u64) {
        self.evict(now_ms);
        *self.tool_call_counts.entry(tool_hash).or_insert(0) += 1;
        self.action_timestamps.push(now_ms);
    }

    /// Increment the credential findings counter.
    pub fn record_credential_finding(&mut self) {
        self.credential_findings_count += 1;
    }

    /// Return the current credential findings count.
    pub fn credential_findings_count(&self) -> u32 {
        self.credential_findings_count
    }

    /// Reset credential findings counter (called after window rotation).
    pub fn reset_credential_findings(&mut self) {
        self.credential_findings_count = 0;
    }

    /// Return the number of actions currently in the window.
    pub fn action_count(&self) -> usize {
        self.action_timestamps.len()
    }

    /// Return the number of calls for a specific tool+args hash.
    pub fn tool_call_count(&self, tool_hash: u64) -> u32 {
        self.tool_call_counts.get(&tool_hash).copied().unwrap_or(0)
    }

    /// Compute mean and standard deviation of per-bucket action rates.
    ///
    /// The window is divided into [`RATE_BUCKETS`] equal intervals. The rate
    /// (actions per bucket) is computed for each, then mean and stddev are
    /// derived. Returns `(0.0, 0.0)` if the window has fewer than 2 actions.
    pub fn action_mean_stddev(&self) -> (f64, f64) {
        if self.action_timestamps.len() < 2 {
            return (0.0, 0.0);
        }

        let earliest = self.action_timestamps[0];
        let latest = *self.action_timestamps.last().unwrap();
        let span = latest.saturating_sub(earliest);
        if span == 0 {
            return (self.action_timestamps.len() as f64, 0.0);
        }

        let bucket_ms = span / RATE_BUCKETS;
        if bucket_ms == 0 {
            return (self.action_timestamps.len() as f64, 0.0);
        }

        let mut buckets = vec![0u32; RATE_BUCKETS as usize];
        for &ts in &self.action_timestamps {
            let idx = ((ts - earliest) / bucket_ms).min(RATE_BUCKETS - 1) as usize;
            buckets[idx] += 1;
        }

        let n = buckets.len() as f64;
        let mean = buckets.iter().map(|&c| c as f64).sum::<f64>() / n;
        let variance = buckets.iter().map(|&c| (c as f64 - mean).powi(2)).sum::<f64>() / n;
        let stddev = variance.sqrt();

        (mean, stddev)
    }

    /// Evict all entries older than `now_ms - window_ms`.
    pub fn evict(&mut self, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(self.window_ms);
        self.action_timestamps.retain(|&ts| ts >= cutoff);
        // Tool call counts are cumulative within the window; reset when
        // the window has fully rotated (no actions remain).
        if self.action_timestamps.is_empty() {
            self.tool_call_counts.clear();
            self.credential_findings_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_baseline_has_zero_action_count() {
        let b = AgentBaseline::new(3600);
        assert_eq!(b.action_count(), 0);
    }

    #[test]
    fn empty_baseline_returns_zero_mean_stddev() {
        let b = AgentBaseline::new(3600);
        let (mean, stddev) = b.action_mean_stddev();
        assert!((mean - 0.0).abs() < f64::EPSILON);
        assert!((stddev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn record_action_increases_count() {
        let mut b = AgentBaseline::new(3600);
        b.record_action(1000);
        b.record_action(2000);
        b.record_action(3000);
        assert_eq!(b.action_count(), 3);
    }

    #[test]
    fn evict_removes_old_entries() {
        let mut b = AgentBaseline::new(10); // 10 second window = 10_000 ms
        b.record_action(1000);
        b.record_action(5000);
        b.record_action(12000);
        // now=12000, window=10000ms, cutoff=2000 → ts=1000 evicted
        assert_eq!(b.action_count(), 2);
    }

    #[test]
    fn evict_clears_tool_counts_when_empty() {
        let mut b = AgentBaseline::new(1); // 1 second window
        b.record_tool_call(42, 1000);
        assert_eq!(b.tool_call_count(42), 1);
        // Evict everything: now=10000, window=1000ms, cutoff=9000
        b.evict(10000);
        assert_eq!(b.tool_call_count(42), 0);
        assert_eq!(b.action_count(), 0);
    }

    #[test]
    fn tool_call_count_tracks_per_hash() {
        let mut b = AgentBaseline::new(3600);
        b.record_tool_call(1, 1000);
        b.record_tool_call(1, 2000);
        b.record_tool_call(2, 3000);
        assert_eq!(b.tool_call_count(1), 2);
        assert_eq!(b.tool_call_count(2), 1);
        assert_eq!(b.tool_call_count(99), 0);
    }

    #[test]
    fn credential_finding_tracking() {
        let mut b = AgentBaseline::new(3600);
        assert_eq!(b.credential_findings_count(), 0);
        b.record_credential_finding();
        b.record_credential_finding();
        assert_eq!(b.credential_findings_count(), 2);
        b.reset_credential_findings();
        assert_eq!(b.credential_findings_count(), 0);
    }

    #[test]
    fn mean_stddev_uniform_distribution() {
        let mut b = AgentBaseline::new(3600);
        // Insert 120 actions evenly spread over 12 seconds (1 per 100ms).
        // Each of the 12 buckets should get ~10 actions → stddev ≈ 0.
        for i in 0..120 {
            b.record_action(1000 + i * 100);
        }
        let (mean, stddev) = b.action_mean_stddev();
        assert!((mean - 10.0).abs() < 1.0, "mean should be ~10, got {mean}");
        assert!(stddev < 2.0, "stddev should be near 0 for uniform, got {stddev}");
    }

    #[test]
    fn mean_stddev_spike_distribution() {
        let mut b = AgentBaseline::new(3600);
        // Insert 10 actions in the first bucket, 0 in the rest.
        for i in 0..10 {
            b.record_action(1000 + i);
        }
        // Add 1 action far away to create a span.
        b.record_action(13000);
        let (mean, stddev) = b.action_mean_stddev();
        // Most buckets are 0, one has ~10, one has 1 → high stddev.
        assert!(stddev > 1.0, "stddev should be high for spiked distribution, got {stddev}");
        assert!(mean > 0.0, "mean should be positive, got {mean}");
    }
}
