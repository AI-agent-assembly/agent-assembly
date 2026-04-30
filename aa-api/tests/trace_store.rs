//! Unit tests for InMemoryTraceStore.

use aa_api::models::trace::TraceSpan;
use aa_api::trace_store::{InMemoryTraceStore, TraceStore};
use chrono::{TimeZone, Utc};

fn make_span(span_id: &str, operation: &str, start_hour: u32) -> TraceSpan {
    TraceSpan {
        span_id: span_id.to_string(),
        parent_span_id: None,
        operation: operation.to_string(),
        decision: Some("allow".to_string()),
        start_time: Utc.with_ymd_and_hms(2026, 1, 1, start_hour, 0, 0).unwrap(),
        end_time: None,
    }
}

#[test]
fn record_and_retrieve_spans() {
    let store = InMemoryTraceStore::new();
    store
        .record_span("session-1", "agent-1", make_span("s1", "llm_call", 10))
        .unwrap();
    store
        .record_span("session-1", "agent-1", make_span("s2", "tool_call", 11))
        .unwrap();

    let trace = store.get_trace("session-1").unwrap().expect("session should exist");
    assert_eq!(trace.agent_id, "agent-1");
    assert_eq!(trace.spans.len(), 2);
    assert_eq!(trace.spans[0].span_id, "s1");
    assert_eq!(trace.spans[1].span_id, "s2");
}

#[test]
fn get_trace_returns_none_for_unknown_session() {
    let store = InMemoryTraceStore::new();
    assert!(store.get_trace("nonexistent").unwrap().is_none());
}

#[test]
fn list_sessions_returns_most_recent_first() {
    let store = InMemoryTraceStore::new();
    store
        .record_span("session-a", "agent-1", make_span("s1", "op1", 10))
        .unwrap();
    store
        .record_span("session-b", "agent-1", make_span("s2", "op2", 11))
        .unwrap();
    store
        .record_span("session-c", "agent-1", make_span("s3", "op3", 12))
        .unwrap();

    let sessions = store.list_sessions(10).unwrap();
    assert_eq!(sessions, vec!["session-c", "session-b", "session-a"]);
}

#[test]
fn list_sessions_respects_limit() {
    let store = InMemoryTraceStore::new();
    for i in 0..5 {
        store
            .record_span(&format!("s-{i}"), "agent-1", make_span("span", "op", 10))
            .unwrap();
    }

    let sessions = store.list_sessions(2).unwrap();
    assert_eq!(sessions.len(), 2);
}

#[test]
fn bounded_capacity_evicts_oldest_session() {
    let store = InMemoryTraceStore::with_capacity(3, 100);

    store
        .record_span("session-1", "agent-1", make_span("s1", "op", 10))
        .unwrap();
    store
        .record_span("session-2", "agent-1", make_span("s2", "op", 11))
        .unwrap();
    store
        .record_span("session-3", "agent-1", make_span("s3", "op", 12))
        .unwrap();
    // This should evict session-1.
    store
        .record_span("session-4", "agent-1", make_span("s4", "op", 13))
        .unwrap();

    assert!(store.get_trace("session-1").unwrap().is_none(), "session-1 should be evicted");
    assert!(store.get_trace("session-4").unwrap().is_some(), "session-4 should exist");

    let sessions = store.list_sessions(10).unwrap();
    assert_eq!(sessions.len(), 3);
}

#[test]
fn bounded_spans_per_session_evicts_oldest_span() {
    let store = InMemoryTraceStore::with_capacity(100, 2);

    store
        .record_span("session-1", "agent-1", make_span("s1", "first", 10))
        .unwrap();
    store
        .record_span("session-1", "agent-1", make_span("s2", "second", 11))
        .unwrap();
    // This should evict span s1.
    store
        .record_span("session-1", "agent-1", make_span("s3", "third", 12))
        .unwrap();

    let trace = store.get_trace("session-1").unwrap().unwrap();
    assert_eq!(trace.spans.len(), 2);
    assert_eq!(trace.spans[0].span_id, "s2");
    assert_eq!(trace.spans[1].span_id, "s3");
}

#[test]
fn get_trace_returns_spans_sorted_by_start_time() {
    let store = InMemoryTraceStore::new();

    // Insert out of chronological order.
    store
        .record_span("session-1", "agent-1", make_span("late", "op2", 15))
        .unwrap();
    store
        .record_span("session-1", "agent-1", make_span("early", "op1", 9))
        .unwrap();
    store
        .record_span("session-1", "agent-1", make_span("mid", "op3", 12))
        .unwrap();

    let trace = store.get_trace("session-1").unwrap().unwrap();
    assert_eq!(trace.spans[0].span_id, "early");
    assert_eq!(trace.spans[1].span_id, "mid");
    assert_eq!(trace.spans[2].span_id, "late");
}
