//! Circular replay buffer for reconnecting WebSocket clients.
//!
//! Stores the most recent [`MAX_CAPACITY`] governance events so that a
//! client reconnecting with `since=<event_id>` can catch up on events
//! it missed while disconnected.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::models::{EventId, GovernanceEvent};

/// Maximum number of events retained in the replay buffer.
const MAX_CAPACITY: usize = 1_000;

/// Thread-safe circular buffer of recent governance events.
#[derive(Debug, Clone)]
pub struct ReplayBuffer {
    inner: Arc<Mutex<VecDeque<GovernanceEvent>>>,
}

impl ReplayBuffer {
    /// Create a new empty replay buffer.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_CAPACITY))),
        }
    }

    /// Push an event into the buffer, evicting the oldest if at capacity.
    pub fn push(&self, event: GovernanceEvent) {
        let mut buf = self.inner.lock().expect("replay buffer lock poisoned");
        if buf.len() >= MAX_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(event);
    }

    /// Return all events with an id strictly greater than `since_id`.
    ///
    /// Returns an empty vec if `since_id` is beyond the newest event
    /// or the buffer is empty.
    pub fn events_since(&self, since_id: EventId) -> Vec<GovernanceEvent> {
        let buf = self.inner.lock().expect("replay buffer lock poisoned");
        buf.iter().filter(|e| e.id > since_id).cloned().collect()
    }
}

impl Default for ReplayBuffer {
    fn default() -> Self {
        Self::new()
    }
}
