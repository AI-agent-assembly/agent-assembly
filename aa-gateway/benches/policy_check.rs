//! Criterion benchmarks for PolicyService CheckAction RPC latency.
//!
//! Measures end-to-end gRPC round-trip (serialize → transport → evaluate → respond)
//! across three representative payload variants.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_placeholder(_c: &mut Criterion) {
    // Will be replaced in the next commit with real benchmarks.
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
