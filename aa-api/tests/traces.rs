//! Integration tests for the trace endpoint.

mod common;

use aa_api::models::trace::{TraceResponse, TraceSpan};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use tower::ServiceExt;

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

#[tokio::test]
async fn get_trace_returns_404_for_unknown_session() {
    let app = common::test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/traces/nonexistent-session-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_trace_returns_200_with_spans() {
    let state = common::test_state();

    // Pre-populate the trace store.
    state
        .trace_store
        .record_span("session-abc", "agent-42", make_span("span-1", "llm_call", 10))
        .unwrap();
    state
        .trace_store
        .record_span("session-abc", "agent-42", make_span("span-2", "tool_call", 11))
        .unwrap();

    let app = aa_api::server::build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/traces/session-abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let trace: TraceResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(trace.session_id, "session-abc");
    assert_eq!(trace.agent_id, "agent-42");
    assert_eq!(trace.spans.len(), 2);
    assert_eq!(trace.spans[0].span_id, "span-1");
    assert_eq!(trace.spans[0].operation, "llm_call");
    assert_eq!(trace.spans[1].span_id, "span-2");
    assert_eq!(trace.spans[1].operation, "tool_call");
}

#[tokio::test]
async fn get_trace_spans_are_ordered_chronologically() {
    let state = common::test_state();

    // Insert spans out of chronological order.
    state
        .trace_store
        .record_span("session-order", "agent-1", make_span("late", "op3", 15))
        .unwrap();
    state
        .trace_store
        .record_span("session-order", "agent-1", make_span("early", "op1", 9))
        .unwrap();
    state
        .trace_store
        .record_span("session-order", "agent-1", make_span("mid", "op2", 12))
        .unwrap();

    let app = aa_api::server::build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/traces/session-order")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let trace: TraceResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(trace.spans.len(), 3);
    assert_eq!(trace.spans[0].span_id, "early");
    assert_eq!(trace.spans[1].span_id, "mid");
    assert_eq!(trace.spans[2].span_id, "late");
}
