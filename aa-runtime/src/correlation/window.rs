//! In-memory sliding time window for correlation events.
//!
//! Events are stored in a `BTreeMap<u64, Vec<CorrelationEvent>>` keyed by
//! timestamp (milliseconds). TTL eviction removes events older than the
//! configured window duration.

use std::collections::BTreeMap;

use super::event::{ActionEvent, CorrelationEvent, IntentEvent};

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

    /// Returns all intent events in the window, ordered by timestamp.
    pub fn intents(&self) -> Vec<&IntentEvent> {
        self.events
            .values()
            .flat_map(|bucket| bucket.iter())
            .filter_map(|e| match e {
                CorrelationEvent::Intent(intent) => Some(intent),
                _ => None,
            })
            .collect()
    }

    /// Returns all action events in the window, ordered by timestamp.
    pub fn actions(&self) -> Vec<&ActionEvent> {
        self.events
            .values()
            .flat_map(|bucket| bucket.iter())
            .filter_map(|e| match e {
                CorrelationEvent::Action(action) => Some(action),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation::event::{ActionEvent, IntentEvent};
    use uuid::Uuid;

    fn make_intent_event(ts: u64) -> CorrelationEvent {
        CorrelationEvent::Intent(IntentEvent {
            event_id: Uuid::new_v4(),
            timestamp_ms: ts,
            pid: 1,
            intent_text: "test".to_string(),
            action_keyword: "test".to_string(),
        })
    }

    fn make_action_event(ts: u64) -> CorrelationEvent {
        CorrelationEvent::Action(ActionEvent {
            event_id: Uuid::new_v4(),
            timestamp_ms: ts,
            pid: 1,
            syscall: "unlink".to_string(),
            details: "/tmp/foo".to_string(),
        })
    }

    #[test]
    fn new_window_is_empty() {
        let w = SlidingWindow::new(5000, 100);
        assert!(w.is_empty());
        assert_eq!(w.len(), 0);
    }

    #[test]
    fn insert_increases_len() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(1000));
        assert_eq!(w.len(), 1);
        w.insert(make_action_event(1000));
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn evict_removes_old_events() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(1000));
        w.insert(make_action_event(7000));
        // now_ms=7000, window=5000 → cutoff=2000 → event at 1000 evicted
        w.evict(7000);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn evict_keeps_events_within_window() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(3000));
        w.insert(make_action_event(4000));
        // now_ms=7000, window=5000 → cutoff=2000 → both kept
        w.evict(7000);
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn max_size_returns_configured_value() {
        let w = SlidingWindow::new(5000, 42);
        assert_eq!(w.max_size(), 42);
    }

    #[test]
    fn intents_returns_only_intent_events() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(1000));
        w.insert(make_action_event(2000));
        w.insert(make_intent_event(3000));
        let intents = w.intents();
        assert_eq!(intents.len(), 2);
        assert_eq!(intents[0].timestamp_ms, 1000);
        assert_eq!(intents[1].timestamp_ms, 3000);
    }

    #[test]
    fn actions_returns_only_action_events() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(1000));
        w.insert(make_action_event(2000));
        w.insert(make_action_event(3000));
        let actions = w.actions();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].timestamp_ms, 2000);
        assert_eq!(actions[1].timestamp_ms, 3000);
    }

    #[test]
    fn intents_empty_when_only_actions() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_action_event(1000));
        assert!(w.intents().is_empty());
    }

    #[test]
    fn actions_empty_when_only_intents() {
        let mut w = SlidingWindow::new(5000, 100);
        w.insert(make_intent_event(1000));
        assert!(w.actions().is_empty());
    }
}
