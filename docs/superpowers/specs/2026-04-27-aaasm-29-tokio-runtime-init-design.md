# AAASM-29: `aa-runtime` Bootstrap — Tokio Runtime Init & Graceful Shutdown Design Spec

**Date:** 2026-04-27
**Ticket:** [AAASM-29](https://lightning-dust-mite.atlassian.net/browse/AAASM-29)
**Epic:** [AAASM-3](https://lightning-dust-mite.atlassian.net/browse/AAASM-3) — Async Sidecar Runtime
**Branch:** `v0.0.1/AAASM-29/tokio_runtime_init`

---

## Purpose

Bootstrap the `aa-runtime` crate as a Tokio-based binary with structured concurrency, configurable
runtime parameters, and graceful shutdown handling. This is the foundation (F6) upon which the
IPC server (AAASM-30), event pipeline (AAASM-31), health endpoint (AAASM-32), and Docker image
(AAASM-33) are all built. It also establishes the full epic module layout as stubs so subsequent
tickets slot in cleanly.

---

## Scope

### In scope

- `aa-runtime/Cargo.toml` — add `tokio-util`, `tracing`, `tracing-subscriber` dependencies
- `aa-runtime/src/lib.rs` — module declarations + public re-export of `run()`
- `aa-runtime/src/main.rs` — binary entry point: `#[tokio::main]`, tracing init, calls `run()`
- `aa-runtime/src/config.rs` — `RuntimeConfig` struct, `from_env()` constructor
- `aa-runtime/src/runtime.rs` — `async fn run()`: `TaskTracker` + `CancellationToken` lifecycle
- `aa-runtime/src/lifecycle.rs` — `wait_for_shutdown_signal()`: SIGTERM + SIGINT via `tokio::signal`
- `aa-runtime/src/ipc/mod.rs` — empty stub (for AAASM-30)
- `aa-runtime/src/pipeline/mod.rs` — empty stub (for AAASM-31)
- `aa-runtime/src/health.rs` — empty stub (for AAASM-32)

### Out of scope

- IPC server implementation (AAASM-30)
- Event aggregation pipeline (AAASM-31)
- Health check / metrics HTTP server (AAASM-32)
- Docker image (AAASM-33)
- Any changes outside `aa-runtime/`

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  main.rs                                                    │
│  #[tokio::main(worker_threads = N)]                         │
│  - init_tracing()                                           │
│  - RuntimeConfig::from_env()                                │
│  - aa_runtime::run(config).await                            │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│  runtime.rs  async fn run(config: RuntimeConfig)            │
│  - TaskTracker::new()                                        │
│  - CancellationToken::new()                                  │
│  - (future: spawn ipc, pipeline, health via tracker)        │
│  - lifecycle::wait_for_shutdown_signal().await               │
│  - token.cancel()                                           │
│  - tracker.close() + timeout wait                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
  lifecycle.rs      ipc/mod.rs    pipeline/mod.rs   health.rs
  (SIGTERM/SIGINT)  (stub→AAASM-30) (stub→AAASM-31) (stub→AAASM-32)
```

---

## Design Decisions

### Structured concurrency via `TaskTracker` + `CancellationToken`

Three options evaluated:

| Option | Description | Decision |
|--------|-------------|----------|
| **A — `TaskTracker` + `CancellationToken`** | `tokio-util` tracker closes after all tasks complete; token propagates cancellation | **Selected** |
| B — `JoinSet` | tokio built-in, no extra dep | Rejected — no cooperative cancellation; tasks must be aborted, not drained |
| C — raw `Arc<AtomicBool>` shutdown flag | No extra dep, simple | Rejected — no structured tracking; fire-and-forget risk |

**Rationale:** Ticket explicitly requires `TaskTracker` + `CancellationToken`. All spawned tasks receive a cloned token and must check `token.cancelled()` at yield points. This prevents premature process exit during in-flight event drain.

### Signal handling via `tokio::signal`

`tokio::signal` is already included via `features = ["full"]`. No new dependency needed.
`wait_for_shutdown_signal()` races SIGTERM (`unix::signal`) and SIGINT (`ctrl_c`) using `tokio::select!`.
On Windows, only `ctrl_c` is available — `#[cfg(unix)]` guards the SIGTERM branch.

### `RuntimeConfig` loaded from environment

`RuntimeConfig::from_env()` reads env vars with typed defaults:
- `AA_RUNTIME_WORKER_THREADS` → `usize`, default: `num_cpus` (tokio default)
- `AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS` → `u64`, default: `30`

No CLI arg parser needed for this ticket. Config is a plain struct, fully testable without env side effects.

### Tracing init in `main.rs`, not in `run()`

`tracing-subscriber` global subscriber must be initialised exactly once, before any async work.
Placing it in `main()` (sync) avoids races and keeps `run()` free of global state side effects.
`RUST_LOG` env var is honoured by the `EnvFilter` layer.

---

## New Dependencies

```toml
# aa-runtime/Cargo.toml additions
tokio-util         = { version = "0.7", features = ["rt"] }
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

---

## File Map

| File | Status | Responsibility |
|------|--------|----------------|
| `aa-runtime/Cargo.toml` | Modify | Add 3 new dependencies |
| `aa-runtime/src/lib.rs` | Modify | Module declarations + `pub use runtime::run` |
| `aa-runtime/src/main.rs` | Create | Binary entry: tracing init + `run()` call |
| `aa-runtime/src/config.rs` | Create | `RuntimeConfig` struct + `from_env()` |
| `aa-runtime/src/runtime.rs` | Create | `async fn run()`: structured concurrency + shutdown |
| `aa-runtime/src/lifecycle.rs` | Create | `wait_for_shutdown_signal()`: SIGTERM + SIGINT |
| `aa-runtime/src/ipc/mod.rs` | Create | Empty stub for AAASM-30 |
| `aa-runtime/src/pipeline/mod.rs` | Create | Empty stub for AAASM-31 |
| `aa-runtime/src/health.rs` | Create | Empty stub for AAASM-32 |

---

## Commit Plan (23 atomic commits)

### Phase 1 — Dependencies

| # | Commit | File change |
|---|--------|-------------|
| 1 | `🔧 (aa-runtime): Add tokio-util dependency with rt feature` | `Cargo.toml` |
| 2 | `🔧 (aa-runtime): Add tracing dependency` | `Cargo.toml` |
| 3 | `🔧 (aa-runtime): Add tracing-subscriber with json and env-filter features` | `Cargo.toml` |

### Phase 2 — Module scaffolding

| # | Commit | File change |
|---|--------|-------------|
| 4 | `✨ (aa-runtime): Add config module declaration to lib.rs` | `lib.rs` + empty `config.rs` |
| 5 | `✨ (aa-runtime): Add runtime module declaration to lib.rs` | `lib.rs` + empty `runtime.rs` |
| 6 | `✨ (aa-runtime): Add lifecycle module declaration to lib.rs` | `lib.rs` + empty `lifecycle.rs` |
| 7 | `✨ (aa-runtime): Add ipc stub module for AAASM-30` | `lib.rs` + `ipc/mod.rs` |
| 8 | `✨ (aa-runtime): Add pipeline stub module for AAASM-31` | `lib.rs` + `pipeline/mod.rs` |
| 9 | `✨ (aa-runtime): Add health stub module for AAASM-32` | `lib.rs` + `health.rs` |
| 10 | `✨ (aa-runtime): Add main.rs binary entry point stub` | `main.rs` |

### Phase 3 — `RuntimeConfig`

| # | Commit | File change |
|---|--------|-------------|
| 11 | `✨ (aa-runtime/config): Add RuntimeConfig struct with worker_threads field` | `config.rs` |
| 12 | `✨ (aa-runtime/config): Add shutdown_timeout_secs field to RuntimeConfig` | `config.rs` |
| 13 | `✨ (aa-runtime/config): Add RuntimeConfig::from_env() constructor` | `config.rs` |

### Phase 4 — `lifecycle.rs`

| # | Commit | File change |
|---|--------|-------------|
| 14 | `✨ (aa-runtime/lifecycle): Add wait_for_shutdown_signal() async function` | `lifecycle.rs` |
| 15 | `✨ (aa-runtime/lifecycle): Add SIGTERM handler inside shutdown signal listener` | `lifecycle.rs` |
| 16 | `✨ (aa-runtime/lifecycle): Add SIGINT handler inside shutdown signal listener` | `lifecycle.rs` |

### Phase 5 — `runtime.rs`

| # | Commit | File change |
|---|--------|-------------|
| 17 | `✨ (aa-runtime/runtime): Add run() async function skeleton` | `runtime.rs` |
| 18 | `✨ (aa-runtime/runtime): Add TaskTracker and CancellationToken init in run()` | `runtime.rs` |
| 19 | `✨ (aa-runtime/runtime): Add graceful shutdown sequence with timeout in run()` | `runtime.rs` |

### Phase 6 — `main.rs` wiring

| # | Commit | File change |
|---|--------|-------------|
| 20 | `✨ (aa-runtime): Wire tokio::main entry with configurable worker_threads` | `main.rs` |
| 21 | `✨ (aa-runtime): Initialize tracing JSON subscriber in main` | `main.rs` |

### Phase 7 — Tests

| # | Commit | File change |
|---|--------|-------------|
| 22 | `✅ (aa-runtime/config): Add unit tests for RuntimeConfig::from_env()` | `config.rs` |
| 23 | `✅ (aa-runtime/runtime): Add integration test for graceful shutdown under synthetic load` | `runtime.rs` |

---

## Acceptance Criteria (from ticket)

- [ ] Runtime starts and logs all component initialization
- [ ] SIGTERM triggers graceful shutdown completing within timeout
- [ ] All spawned tasks complete before process exits
- [ ] Zero panics on normal and graceful shutdown paths
- [ ] Integration test verifies graceful shutdown under synthetic load
