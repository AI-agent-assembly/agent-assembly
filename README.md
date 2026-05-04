# agent-assembly

> Governance-native runtime for AI agents — open-source core.

[![CI](https://github.com/AI-agent-assembly/agent-assembly/actions/workflows/ci.yml/badge.svg)](https://github.com/AI-agent-assembly/agent-assembly/actions/workflows/ci.yml)
[![Docs](https://github.com/AI-agent-assembly/agent-assembly/actions/workflows/docs.yml/badge.svg)](https://github.com/AI-agent-assembly/agent-assembly/actions/workflows/docs.yml)
[![codecov](https://codecov.io/gh/AI-agent-assembly/agent-assembly/branch/master/graph/badge.svg)](https://codecov.io/gh/AI-agent-assembly/agent-assembly)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/badge/crates.io-unpublished-lightgrey)](https://crates.io/)


## Overview

`agent-assembly` is the core runtime that brings governance to AI agents at scale. It provides a three-layer interception model — eBPF kernel hooks, a sidecar proxy, and an SDK shim — backed by a policy engine and audit trail.

## Crate Map

The Cargo workspace declares **14 members** in the top-level `Cargo.toml`. Two additional eBPF-target crates live alongside but are intentionally outside the workspace because they compile for the `bpfel-unknown-none` target.

### Workspace members

| Crate | Role |
|---|---|
| `aa-core` | Pure logic, `no_std`-compatible domain types and traits |
| `aa-proto` | Protobuf message types — single source of truth for the wire format |
| `aa-runtime` | Tokio async runtime wrapper and agent lifecycle |
| `aa-ebpf` | eBPF orchestrator (loads probes/programs via `aya-build`) |
| `aa-ebpf-common` | Shared types between user-space and eBPF programs |
| `aa-proxy` | Sidecar HTTPS interception proxy (MitM with per-host CA) |
| `aa-ffi-python` | Python FFI bindings via PyO3 |
| `aa-ffi-node` | Node.js FFI bindings via napi-rs |
| `aa-ffi-go` | Go FFI bindings via cgo |
| `aa-wasm` | WebAssembly target via wasm-bindgen |
| `aa-gateway` | Control plane — policy enforcement, agent registry, budget tracking |
| `aa-api` | HTTP presentation layer with OpenAPI spec generation (utoipa) |
| `aa-cli` | `aasm` command-line tool |
| `conformance` | Cross-SDK protocol conformance test harness |

### Out-of-workspace eBPF target crates

These two are built by `aa-ebpf/build.rs` (via `aya-build`) for the BPF target — they are not part of the host workspace and cannot be selected with `cargo -p`:

| Crate | Role |
|---|---|
| `aa-ebpf-probes` | Userspace probe loaders (uprobes for SSL libraries) |
| `aa-ebpf-programs` | eBPF programs compiled to BPF bytecode (`bpfel-unknown-none`) |

## Project Status

🚧 **Alpha — v0.0.1** — API is not stable. Do not use in production.

## Requirements

- Rust stable (≥ 1.75)
- `protoc` — Protocol Buffers compiler (`brew install protobuf` on macOS, `apt-get install protobuf-compiler` on Debian/Ubuntu); required by `aa-proto` and `aa-gateway` build scripts
- [cargo-nextest](https://nexte.st/) for running tests
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) for dependency checks
- [Lefthook](https://github.com/evilmartians/lefthook) for git hooks
- **Linux only**: `pkg-config` and `libssl-dev` (or `openssl-devel` on RHEL-family) for native TLS in `aa-proxy`; eBPF crates additionally require a recent kernel with BTF and a nightly Rust toolchain (see `aa-ebpf/README.md`)

## Getting Started

```bash
# Clone
git clone https://github.com/AI-agent-assembly/agent-assembly.git
cd agent-assembly

# Install git hooks
lefthook install

# Build all crates
cargo build --workspace

# Run tests
cargo nextest run --workspace
```

## Quickstart — sidecar + test agent

Run `aa-runtime` as a sidecar against a placeholder agent using the [`examples/docker-compose`](examples/docker-compose/) stack:

```bash
# 1. Build the workspace (first time only)
cargo build --workspace --exclude aa-ebpf

# 2. Launch the sidecar + a stub agent container
cd examples/docker-compose
AA_API_KEY=dev-local-key docker compose up
```

The sidecar exposes:

- The agent IPC socket at `/tmp/aa-runtime-my-agent-001.sock`
- Health and metrics on `http://localhost:8080`

To exercise it without Docker, run the gateway and CLI directly:

```bash
# Terminal A — start the gateway
cargo run -p aa-gateway

# Terminal B — confirm registry + topology via the aasm CLI
cargo run -p aa-cli -- topology
```

Replace the `agent-stub` service in `examples/docker-compose/docker-compose.yml` with your own SDK-based agent image once `python-sdk`, `node-sdk`, or `go-sdk` is wired into your project.

## Repository Layout

```
agent-assembly/
├── aa-core/             # Domain types (no_std)
├── aa-proto/            # Protobuf message types (wire format)
├── aa-runtime/          # Async runtime + agent lifecycle
├── aa-ebpf/             # eBPF orchestrator (workspace member)
├── aa-ebpf-common/      # Shared user/kernel types (workspace member)
├── aa-ebpf-probes/      # Userspace probe loaders (out-of-workspace, BPF target)
├── aa-ebpf-programs/    # eBPF programs (out-of-workspace, BPF target)
├── aa-proxy/            # Sidecar HTTPS proxy
├── aa-ffi-python/       # Python bindings (PyO3)
├── aa-ffi-node/         # Node bindings (napi-rs)
├── aa-ffi-go/           # Go bindings (cgo)
├── aa-wasm/             # WASM target
├── aa-gateway/          # Control plane (policy, registry, budget)
├── aa-api/              # HTTP API + OpenAPI
├── aa-cli/              # CLI tool (aasm)
├── conformance/         # Protocol conformance test harness
├── proto/               # Protobuf source (.proto files)
├── openapi/             # Generated OpenAPI v1 spec
├── schemas/             # JSON schemas (compatibility matrix)
├── dashboard/           # Community web UI (React + TypeScript)
├── docs/                # mdBook contributor documentation
└── policy-examples/     # Reference governance policies
```

## Documentation

The contributor-facing documentation is published as an [mdBook](https://rust-lang.github.io/mdBook/). Sources live under `docs/src/`. Build it locally with:

```bash
cargo install --locked --version 0.5.2 mdbook
cargo install --locked --version 0.17.0 mdbook-mermaid
mdbook serve docs --open
```

| Chapter | Description |
|---|---|
| [Introduction](docs/src/README.md) | Book overview and audience |
| [Architecture Overview](docs/src/architecture.md) | Crate dependency graph, three-layer interception, IPC, sidecar lifecycle, policy evaluation |
| [API Reference](docs/src/api-reference.md) | rustdoc generation flow and per-crate API surface map |
| [Compatibility Matrix](docs/src/compatibility.md) | Which `aa-runtime` versions work with which SDK versions |
| [Versioning Policy](docs/src/versioning.md) | Protocol semver rules, breaking-change classification, deprecation lifecycle |
| [Protocol Changelog](docs/src/protocol/CHANGELOG.md) | Wire-protocol change log |
| [Migration Template](docs/src/migration/template.md) | Guidance for moving between protocol versions |
| [Benchmarks — Baseline](docs/src/benchmarks/BASELINE.md) | Performance baseline numbers |
| [Benchmarks — Policy Check p99](docs/src/benchmarks/policy-check-p99.md) | Latency SLA evidence |

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
