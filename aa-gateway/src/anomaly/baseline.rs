//! Per-agent behavioral baseline for anomaly detection.
//!
//! Maintains a sliding window of action counts, tool usage frequency, and
//! connection patterns for each agent. The baseline is used by the detector
//! to identify deviations from normal behavior.
//!
//! Design note: the sliding window pattern from
//! `aa-runtime::correlation::window::SlidingWindow` (BTreeMap-based temporal
//! storage with TTL eviction) is directly reusable here. The anomaly baseline
//! window tracks action rates rather than correlation events, but the
//! time-bucketed insert/evict mechanism is the same.

// TODO(AAASM-137): Implement AgentBaseline struct with:
//   - sliding window of action counts (configurable window, default 1 hour)
//   - per-tool invocation frequency tracking
//   - mean + standard deviation computation for spike detection
//   - connection host histogram for unknown-connection detection
