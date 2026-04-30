//! Session trace storage trait and in-memory implementation.
//!
//! Provides a [`TraceStore`] trait for recording and querying trace spans
//! indexed by session ID, plus an [`InMemoryTraceStore`] backed by `DashMap`.

use std::collections::VecDeque;

use dashmap::DashMap;

use crate::models::trace::TraceSpan;

/// Maximum number of sessions retained in the in-memory store.
const DEFAULT_MAX_SESSIONS: usize = 10_000;

/// Maximum number of spans retained per session.
const DEFAULT_MAX_SPANS_PER_SESSION: usize = 1_000;

/// Trait for session trace storage.
///
/// Implementations must be safe to share across threads and async tasks.
pub trait TraceStore: Send + Sync {
    /// Record a span for the given session.
    fn record_span(&self, session_id: &str, agent_id: &str, span: TraceSpan) -> Result<(), TraceStoreError>;

    /// Retrieve the full trace for a session, with spans in chronological order.
    fn get_trace(&self, session_id: &str) -> Result<Option<SessionTrace>, TraceStoreError>;

    /// List session IDs with recorded traces, most recent first.
    fn list_sessions(&self, limit: usize) -> Result<Vec<String>, TraceStoreError>;
}

/// Metadata for a stored session trace.
#[derive(Debug, Clone)]
pub struct SessionTrace {
    /// Agent that produced this trace.
    pub agent_id: String,
    /// Ordered list of spans in the session.
    pub spans: Vec<TraceSpan>,
}

/// Errors from trace store operations.
#[derive(Debug, thiserror::Error)]
pub enum TraceStoreError {
    /// An internal storage error.
    #[error("trace store internal error: {0}")]
    Internal(String),
}

/// Thread-safe in-memory trace store backed by `DashMap`.
pub struct InMemoryTraceStore {
    /// Map from session_id to (agent_id, spans).
    sessions: DashMap<String, (String, VecDeque<TraceSpan>)>,
    /// Insertion-ordered session IDs for LRU eviction and listing.
    session_order: std::sync::Mutex<VecDeque<String>>,
    max_sessions: usize,
    max_spans_per_session: usize,
}

impl InMemoryTraceStore {
    /// Create a new in-memory trace store with default capacity limits.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_SESSIONS, DEFAULT_MAX_SPANS_PER_SESSION)
    }

    /// Create a new in-memory trace store with custom capacity limits.
    pub fn with_capacity(max_sessions: usize, max_spans_per_session: usize) -> Self {
        Self {
            sessions: DashMap::new(),
            session_order: std::sync::Mutex::new(VecDeque::with_capacity(max_sessions)),
            max_sessions,
            max_spans_per_session,
        }
    }
}

impl Default for InMemoryTraceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStore for InMemoryTraceStore {
    fn record_span(&self, session_id: &str, agent_id: &str, span: TraceSpan) -> Result<(), TraceStoreError> {
        let is_new_session = !self.sessions.contains_key(session_id);

        if is_new_session {
            // Evict oldest session if at capacity.
            let mut order = self.session_order.lock().expect("session_order lock poisoned");
            if order.len() >= self.max_sessions {
                if let Some(oldest) = order.pop_front() {
                    self.sessions.remove(&oldest);
                }
            }
            order.push_back(session_id.to_string());
        }

        let mut entry = self.sessions.entry(session_id.to_string()).or_insert_with(|| {
            (
                agent_id.to_string(),
                VecDeque::with_capacity(self.max_spans_per_session),
            )
        });

        let (_, spans) = entry.value_mut();
        if spans.len() >= self.max_spans_per_session {
            spans.pop_front();
        }
        spans.push_back(span);

        Ok(())
    }

    fn get_trace(&self, session_id: &str) -> Result<Option<SessionTrace>, TraceStoreError> {
        let Some(entry) = self.sessions.get(session_id) else {
            return Ok(None);
        };

        let (agent_id, spans) = entry.value();
        let mut sorted_spans: Vec<TraceSpan> = spans.iter().cloned().collect();
        sorted_spans.sort_by_key(|s| s.start_time);

        Ok(Some(SessionTrace {
            agent_id: agent_id.clone(),
            spans: sorted_spans,
        }))
    }

    fn list_sessions(&self, limit: usize) -> Result<Vec<String>, TraceStoreError> {
        let order = self.session_order.lock().expect("session_order lock poisoned");
        // Return most recent first.
        Ok(order.iter().rev().take(limit).cloned().collect())
    }
}
