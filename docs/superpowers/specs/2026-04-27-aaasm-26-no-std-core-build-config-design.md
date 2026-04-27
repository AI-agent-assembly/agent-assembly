# AAASM-26: `no_std` Core Build Config for `aa-core` — Design Spec

**Date:** 2026-04-27
**Ticket:** [AAASM-26](https://lightning-dust-mite.atlassian.net/browse/AAASM-26)
**Epic:** [AAASM-2](https://lightning-dust-mite.atlassian.net/browse/AAASM-2) — Runtime Core Foundations
**Sprint:** AAA Sprint 1 (ends 2026-04-30)
**Branch:** `v0.0.1/AAASM-26/no_std_core_build_config`

---

## Purpose

Establish the `no_std` build configuration, feature flags, and conditional compilation
strategy for `aa-core` so all core data structures work in both `std` and `no_std` + `alloc`
environments. This is the foundation that AAASM-22–25 build on.

---

## Scope

### In scope
- `aa-core/Cargo.toml` — feature flags + dependencies
- `aa-core/src/lib.rs` — `no_std` gate, `cfg_if!` setup, conditional imports, module doc
- `aa-core/src/time.rs` — new file: `Timestamp` type (the only concrete type in this ticket)
- `.github/workflows/ci.yml` — new `no-std` matrix job

### Out of scope
- No domain types (AgentContext, PolicyEvaluator, AuditEntry — AAASM-22–25)
- No changes to any other workspace crate
- No `dashboard/` or tooling changes

---

## Design Decisions

### Conditional compilation strategy: `cfg_if!` macro

Three options were evaluated:

| Option | Description | Decision |
|--------|-------------|----------|
| **A — `cfg_if!` macro** | Use `cfg_if` crate for all conditional blocks | **Selected** |
| B — Raw `#[cfg(...)]` | Bare attribute syntax, no extra dep | Rejected — verbose for compound conditions, not ticket-specified |
| C — `compat` re-export module | `src/compat.rs` re-exports the right types | Rejected — premature abstraction, no types exist yet |

**Rationale:** `cfg_if!` is the most readable for feature-combo logic as the codebase grows.
It is explicitly called for in the ticket spec. `cfg-if` has ~500M downloads and is a
de-facto standard in the `no_std` ecosystem.

### `serde` feature scope

Only the `Cargo.toml` declaration is added here (Option A from ticket comment). No derives
or prelude re-exports. This pins the version and establishes the feature name so AAASM-22–25
never need to touch `Cargo.toml`. Individual type tickets add `#[cfg_attr(feature = "serde", ...)]`.

---

## File Changes

### `aa-core/Cargo.toml`

```toml
[dependencies]
cfg-if = "1"
serde  = { version = "1", default-features = false, features = ["derive"], optional = true }

[features]
default = ["std"]
std     = []
alloc   = []
serde   = ["dep:serde"]
```

- `default = ["std"]` — consumers get `std` unless they opt out
- `std` and `alloc` are independent, non-exclusive flags
- `serde = ["dep:serde"]` — Cargo's `dep:` syntax avoids feature/dependency name conflict

### `aa-core/src/lib.rs`

```rust
//! Core domain logic for Agent Assembly.
//!
//! # Feature Flags
//!
//! - `std` (default): enables `std`-dependent convenience impls (e.g. `From<SystemTime>`)
//! - `alloc`: enables heap types (`String`, `Vec`, `BTreeMap`) in `no_std` environments
//! - `serde`: enables `Serialize`/`Deserialize` derives on all core types (added in AAASM-22–25)

#![cfg_attr(not(feature = "std"), no_std)]

cfg_if::cfg_if! {
    if #[cfg(feature = "alloc")] {
        extern crate alloc;
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod time;
```

- `#![cfg_attr(...)]` — crate stays `std` in default builds, silently goes `no_std` when `std` absent
- `cfg_if!` block — establishes the pattern all future type modules follow

### `aa-core/src/time.rs` (new file)

```rust
/// Nanoseconds since the Unix epoch.
///
/// - no_std: caller supplies the value via `Timestamp::from_nanos`
/// - std: use the `From<SystemTime>` convenience impl
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp(u64);   // inner field private — API-stable

impl Timestamp {
    pub const fn from_nanos(nanos: u64) -> Self { ... }
    pub const fn as_nanos(&self) -> u64 { ... }
}

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for Timestamp { ... }
```

- Inner `u64` is private — `from_nanos`/`as_nanos` are the stable API surface
- `From<SystemTime>` is `#[cfg(feature = "std")]` — pattern for all std-only impls
- `cfg_attr` uses fully-qualified `serde::Serialize` — no conditional `use` needed per module
- Tests: one no_std-compatible (round-trip), one `#[cfg(feature = "std")]` (epoch = 0)

### `.github/workflows/ci.yml`

New `no-std` job with 2-target matrix:

```yaml
no-std:
  strategy:
    matrix:
      include:
        - target: wasm32-unknown-unknown
        - target: thumbv7em-none-eabihf
  # builds aa-core only with --no-default-features --features alloc
```

- Only `aa-core` built — no other crate is `no_std` compatible yet
- No `protobuf-compiler` needed — `aa-core` has no proto dependency

---

## Commit Plan (11 commits)

| # | Message | Key change |
|---|---------|------------|
| 1 | `🔧 (aa-core): Add cfg-if dependency` | Add `cfg-if = "1"` to `[dependencies]` |
| 2 | `🔧 (aa-core): Add serde as optional dependency` | Add serde `optional = true, default-features = false` |
| 3 | `🔧 (aa-core): Add std, alloc and serde feature flags` | Add full `[features]` section |
| 4 | `✨ (aa-core): Add no_std conditional crate attribute` | `#![cfg_attr(not(feature = "std"), no_std)]` |
| 5 | `✨ (aa-core): Add cfg_if block for conditional alloc extern` | `cfg_if!` block gating `extern crate alloc` |
| 6 | `✨ (aa-core): Add conditional serde import` | `#[cfg(feature = "serde")] use serde::{...}` |
| 7 | `📝 (aa-core): Document feature flags in crate-level doc comment` | `# Feature Flags` section in module doc |
| 8 | `✨ (aa-core/time): Add Timestamp struct with constructor methods` | New `src/time.rs` + `pub mod time` in lib.rs |
| 9 | `✨ (aa-core/time): Add From<SystemTime> impl under std feature` | `#[cfg(feature = "std")] impl From<SystemTime>` |
| 10 | `✅ (aa-core/time): Add unit tests for Timestamp` | Round-trip test + std From test |
| 11 | `🔧 (ci): Add no_std CI matrix job for wasm32 and thumbv7em` | New CI job, 2-target matrix |

---

## Acceptance Criteria (from ticket)

- [ ] `cargo build --target wasm32-unknown-unknown --no-default-features --features alloc` succeeds
- [ ] All core types available in `no_std` + `alloc` configuration
- [ ] `std` feature adds convenience impls (`From<SystemTime>`) but is not required
- [ ] CI matrix includes at least one `no_std` target
- [ ] Zero use of `std::` outside `#[cfg(feature = "std")]` blocks

---

## Pattern for AAASM-22–25

Each domain type ticket follows this template:

```rust
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentContext { ... }

#[cfg(feature = "std")]
impl SomeStdTrait for AgentContext { ... }
```

No future domain ticket needs to touch `Cargo.toml` — all feature wiring is done here.
