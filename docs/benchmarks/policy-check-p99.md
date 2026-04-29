# PolicyService CheckAction RPC — Latency Benchmark Results

## Environment

| Parameter | Value |
|-----------|-------|
| CPU | Apple M3 Max |
| Memory | 128 GB |
| OS | macOS 26.2 (Darwin) |
| Rust | 1.95.0 (2026-04-14) |
| Tonic | 0.13.1 |
| Transport | TCP loopback (127.0.0.1) |
| Profile | `--release` (optimized) |

## SLA Target

**p99 < 5ms** end-to-end round-trip (serialize + transport + evaluate + respond).

## Criterion Micro-Benchmarks

Reused TCP connection, single client, 100 samples per variant.

| Payload Variant | Description | Mean | Std Dev |
|-----------------|-------------|------|---------|
| `minimal_llm_call` | `LlmCallContext`, no PII | 77.9 us | ~1 us |
| `full_tool_call_1kb` | `ToolCallContext`, ~1KB `args_json` | 82.2 us | ~1 us |
| `worst_case_network` | `NetworkCallContext`, long URL (~400 bytes) | 81.9 us | ~1 us |

## Sustained Load Test (60 seconds)

1,000 req/sec sustained for 60 seconds, 10 concurrent clients, `ToolCallContext` payload.

| Metric | Value | vs SLA |
|--------|-------|--------|
| Total requests | 60,000 | |
| Actual RPS | 999 | |
| **p50** | 144 us | 34x headroom |
| **p95** | 357 us | 14x headroom |
| **p99** | **803 us** | **6.2x headroom** |
| **p999** | 2.65 ms | 1.9x headroom |
| **max** | 10.89 ms | |

## Verdict

**PASS** — p99 latency of 803 us is well under the 5ms SLA target with 6.2x headroom.

The max latency (10.89 ms) exceeds 5ms but this is expected for a single outlier in
60,000 requests on a non-isolated workstation. The p999 (2.65 ms) confirms the tail
is well-bounded for all practical purposes.
