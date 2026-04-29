//! Criterion benchmarks for proxy MitM intercept latency.
//!
//! Measures the time for `Interceptor::intercept()` to detect an LLM API call,
//! extract fields from the response body, scan for credentials, build the
//! pipeline event, and broadcast it.
//!
//! Target: < 5 ms P99  (AAASM-36 AC #5).

use std::time::SystemTime;

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use tokio::sync::broadcast;

use aa_proxy::intercept::detect::LlmApiPattern;
use aa_proxy::intercept::event::ProxyEvent;
use aa_proxy::intercept::Interceptor;

/// Realistic OpenAI chat completion response (~350 bytes).
const OPENAI_RESPONSE_BODY: &str = r#"{"id":"chatcmpl-abc123","object":"chat.completion","created":1714000000,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"Hello! How can I help you today?"},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":9,"total_tokens":21}}"#;

/// Response body containing an embedded credential for redaction benchmarking.
const OPENAI_RESPONSE_WITH_CREDENTIAL: &str = r#"{"id":"chatcmpl-abc123","object":"chat.completion","created":1714000000,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"Your key is sk-proj-aBcDeFgHiJkLmNoPqRsT1234567890abcdef1234567890ab which you should rotate."},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":25,"total_tokens":37}}"#;

fn make_openai_event(body: &str) -> ProxyEvent {
    ProxyEvent {
        agent_id: Some("bench-agent".into()),
        pattern: LlmApiPattern::OpenAi,
        method: "POST".into(),
        path: "/v1/chat/completions".into(),
        request_body: None,
        response_body: Some(Bytes::from(body.to_owned())),
        timestamp: SystemTime::now(),
    }
}

/// Benchmark: intercept an OpenAI response (detect + extract + broadcast).
fn bench_intercept_openai(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (tx, _rx) = broadcast::channel(4096);
    let interceptor = Interceptor::new(tx);
    let event = make_openai_event(OPENAI_RESPONSE_BODY);

    let mut group = c.benchmark_group("intercept");

    group.bench_function("openai_response", |b| {
        b.to_async(&rt).iter(|| async {
            let result = interceptor.intercept(&event).await;
            criterion::black_box(result)
        });
    });

    group.finish();
}

/// Benchmark: intercept with credential scanning + redaction overhead.
fn bench_intercept_with_credential_scan(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (tx, _rx) = broadcast::channel(4096);
    let interceptor = Interceptor::new(tx);
    let event = make_openai_event(OPENAI_RESPONSE_WITH_CREDENTIAL);

    let mut group = c.benchmark_group("intercept");

    group.bench_function("openai_with_credential_redaction", |b| {
        b.to_async(&rt).iter(|| async {
            let result = interceptor.intercept(&event).await;
            criterion::black_box(result)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_intercept_openai, bench_intercept_with_credential_scan);
criterion_main!(benches);
