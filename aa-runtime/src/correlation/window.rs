//! In-memory sliding time window for correlation events.
//!
//! Events are stored in a `BTreeMap<u64, Vec<CorrelationEvent>>` keyed by
//! timestamp (milliseconds). TTL eviction removes events older than the
//! configured window duration.

use std::collections::BTreeMap;

use super::event::CorrelationEvent;

/// A time-ordered sliding window of correlation events.
///
/// Events are indexed by their timestamp in milliseconds. The window supports
/// insertion, TTL-based eviction, and range queries for the correlation
/// algorithm.
#[derive(Debug)]
pub struct SlidingWindow {
    /// Events indexed by timestamp. Multiple events can share the same
    /// millisecond timestamp.
    events: BTreeMap<u64, Vec<CorrelationEvent>>,
    /// Maximum age (in milliseconds) before an event is evicted.
    window_ms: u64,
    /// Maximum total events before force-eviction of the oldest entries.
    max_size: usize,
}

impl SlidingWindow {
    /// Create a new sliding window with the given time window and capacity.
    pub fn new(window_ms: u64, max_size: usize) -> Self {
        Self {
            events: BTreeMap::new(),
            window_ms,
            max_size,
        }
    }

    /// Insert an event into the window.
    pub fn insert(&mut self, event: CorrelationEvent) {
        let ts = event.timestamp_ms();
        self.events.entry(ts).or_default().push(event);
    }

    /// Evict all events older than `now_ms - window_ms`.
    pub fn evict(&mut self, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(self.window_ms);
        // `split_off` returns everything >= cutoff; we keep that and drop the rest.
        self.events = self.events.split_off(&cutoff);
    }

    /// Returns the total number of events currently in the window.
    pub fn len(&self) -> usize {
        self.events.values().map(|v| v.len()).sum()
    }

    /// Returns `true` if the window contains no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the configured maximum capacity.
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}
