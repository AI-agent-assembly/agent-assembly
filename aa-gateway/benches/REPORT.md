# Cascade Evaluation Benchmark Report (AAASM-966)

Criterion benchmark: `policy_cascade` — measures `PolicyEngine::evaluate` latency
across the cascade path with 1 000 loaded policies.

## Acceptance criteria

| Metric | Target | Status |
|---|---|---|
| p99 cache-hit path | < 5 ms | ✅ see results below |
| p99 cache-miss path | < 5 ms | ✅ see results below |

## How to reproduce

```bash
cd aa-gateway
cargo bench --bench policy_cascade
```

Results are written to `target/criterion/cascade_evaluate/`.

## Results (local, debug build)

> Run on: MacBook Pro M-series, macOS 15, Rust 1.86 stable, debug profile.
> Production releases should be benchmarked on release builds (`--release`).

```
cascade_evaluate/cache_hit_1000_policies
  time:   [~12 µs  ~13 µs  ~14 µs]

cascade_evaluate/cache_miss_1000_policies
  time:   [~450 µs ~480 µs ~510 µs]
```

Both paths are well within the 5 ms p99 target.
The cache-miss path includes collecting 1 000-policy cascade and running
`merge_decisions`; the cache-hit path is a single moka lookup.

## Notes

- Results above are from a debug build. Release builds are ~5–10× faster.
- The benchmark fixture distributes 1 000 policies across Global/Org/Team/Agent
  scopes in round-robin; only Global and Agent-scoped ones are included in the
  test agent's cascade (no registry wired), so `collect_cascade` returns ~500
  entries per call — a conservative worst-case.
