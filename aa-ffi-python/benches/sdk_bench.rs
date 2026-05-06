//! Criterion benchmarks for SDK hook overhead.
//!
//! Measures the caller-visible latency of `report_llm_call()` — the time to
//! build an `AuditEvent` with `LlmCallDetail` and `blocking_send` it into the
//! mpsc command channel.  This is the hot path that blocks the Python caller;
//! everything after the channel send is asynchronous.
//!
//! Target: < 2 ms P99  (AAASM-34 AC #6).

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::sync::mpsc;

use aa_proto::assembly::audit::v1::{audit_event, AuditEvent, LlmCallDetail};
use aa_proto::assembly::common::v1::ActionType;

/// Replicate the event-ID generator used by `AssemblyHandle::report_llm_call`.
fn unique_event_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{seq:x}")
}

/// Build an `AuditEvent` with `LlmCallDetail` — identical to the construction
/// inside `AssemblyHandle::report_llm_call`.
fn build_llm_event() -> Box<AuditEvent> {
    let detail = LlmCallDetail {
        model: "gpt-4o".into(),
        prompt_tokens: 150,
        completion_tokens: 80,
        latency_ms: 320,
        provider: "openai".into(),
        ..Default::default()
    };

    Box::new(AuditEvent {
        event_id: unique_event_id(),
        action_type: ActionType::LlmCall.into(),
        detail: Some(audit_event::Detail::LlmCall(detail)),
        ..Default::default()
    })
}

/// **Primary benchmark** — validates the < 2 ms P99 target.
///
/// Measures: event construction + `blocking_send` into a `tokio::sync::mpsc`
/// channel with a background drainer thread consuming events.
fn bench_report_llm_call_channel(c: &mut Criterion) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Box<AuditEvent>>(256);

    // Background drainer — simulates the IPC thread consuming events.
    let _drainer = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let mut rx = cmd_rx;
            while let Some(_event) = rx.recv().await {
                // drain
            }
        });
    });

    c.bench_function("report_llm_call_channel", |b| {
        b.iter(|| {
            let event = build_llm_event();
            let result = cmd_tx.blocking_send(event);
            black_box(result)
        });
    });

    // Dropping cmd_tx closes the channel, which terminates the drainer.
}

criterion_group!(benches, bench_report_llm_call_channel);
criterion_main!(benches);
