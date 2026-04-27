# AAASM-22: `AgentContext` Struct — Design Spec

**Date:** 2026-04-27
**Ticket:** [AAASM-22](https://lightning-dust-mite.atlassian.net/browse/AAASM-22)
**Epic:** [AAASM-2](https://lightning-dust-mite.atlassian.net/browse/AAASM-2) — Runtime Core Foundations
**Sprint:** AAA Sprint 1 (ends 2026-04-30)
**Branch:** `v0.0.1/AAASM-22/feat/agent_context_struct`

---

## Purpose

Implement `AgentContext` — the core identity carrier that flows through every governance event
in the agent-assembly system. Alongside it, define the `AgentId` and `SessionId` newtype
wrappers that will be referenced by all future domain types (AuditEntry, PolicyEvaluator, etc.).

---

## Scope

### In scope
- `aa-core/Cargo.toml` — add `serde_json` dev-dependency
- `aa-core/src/identity.rs` — new file: `AgentId` and `SessionId` newtypes
- `aa-core/src/agent.rs` — new file: `AgentContext` struct
- `aa-core/src/lib.rs` — module declarations and re-exports

### Out of scope
- No changes to CI, other workspace crates, or tooling
- No `Timestamp::now()` method — `AgentContext::now()` uses `Timestamp::from(SystemTime::now())`

---

## Design Decisions

### Module structure: split identity primitives from context (Approach B)

Two options were evaluated:

| Option | Description | Decision |
|--------|-------------|----------|
| **A — Single `src/agent.rs`** | All three types co-located in one file | Rejected |
| **B — `src/identity.rs` + `src/agent.rs`** | Identity newtypes separate from context struct | **Selected** |

**Rationale:** `AgentId` and `SessionId` are stack-only `[u8; 16]` newtypes with no `alloc`
requirement. They will be referenced by every future domain type (AuditEntry, PolicyEvaluator,
etc.). Separating them ensures those future types can import identifiers without pulling in
`AgentContext`'s `alloc` dependency. Import paths are semantically honest:
`use crate::identity::AgentId` does not imply "I need agent context."

### `started_at` type: `crate::time::Timestamp`

The `started_at` field uses `crate::time::Timestamp` (established in AAASM-26), not raw `u64`.
Rationale: type safety (prevents silent unit mismatches), design validation of AAASM-26's
abstraction, and consistency across all future time fields in the codebase. Zero runtime cost —
`Timestamp` is a `#[repr(transparent)]`-equivalent newtype over `u64`.

### `AgentContext` alloc gating: entire struct gated on `#[cfg(feature = "alloc")]`

`AgentContext` requires `alloc` for `BTreeMap<&'static str, String>` (metadata). The correct
approach is to gate the entire type, not just the metadata field, because:
- There is no real consumer of `AgentContext` on a heap-free target
- Pure `no_std` without `alloc` still gets `Timestamp`, error enums, and other heap-free primitives
- The `no_std` CI target (`thumbv7em`) verifies build hygiene, not MCU deployability

---

## File Changes

### `aa-core/Cargo.toml`

```toml
[dev-dependencies]
serde_json = "1"
```

### `aa-core/src/identity.rs` (new file)

```rust
/// Stable identifier for an agent — UUID v4 as raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentId([u8; 16]);

/// Per-execution session identifier — UUID v4 as raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionId([u8; 16]);

impl AgentId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self { Self(bytes) }
    pub const fn as_bytes(&self) -> &[u8; 16] { &self.0 }
}

impl SessionId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self { Self(bytes) }
    pub const fn as_bytes(&self) -> &[u8; 16] { &self.0 }
}
```

- Both types are stack-only — no `alloc` gate needed
- `Copy` is correct: `[u8; 16]` is small and cheaply duplicable
- Inner field is private — `from_bytes`/`as_bytes` are the stable API surface

### `aa-core/src/agent.rs` (new file)

```rust
#[cfg(feature = "alloc")]
use alloc::{collections::BTreeMap, string::String};
use crate::{identity::{AgentId, SessionId}, time::Timestamp};

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentContext {
    /// Stable identifier for the agent (UUID v4 bytes).
    pub agent_id:   AgentId,
    /// Per-execution session identifier (UUID v4 bytes).
    pub session_id: SessionId,
    /// OS process ID of the agent process.
    pub pid:        u32,
    /// Nanoseconds since Unix epoch when this context was created.
    pub started_at: Timestamp,
    /// Extensible key-value metadata.
    pub metadata:   BTreeMap<&'static str, String>,
}

#[cfg(all(feature = "alloc", feature = "std"))]
impl AgentContext {
    /// Construct a new `AgentContext` stamped at the current wall-clock time.
    pub fn now(agent_id: AgentId, session_id: SessionId, pid: u32) -> Self {
        Self {
            started_at: Timestamp::from(std::time::SystemTime::now()),
            agent_id,
            session_id,
            pid,
            metadata: BTreeMap::new(),
        }
    }
}
```

### `aa-core/src/lib.rs` additions

```rust
pub mod identity;
pub mod agent;

pub use identity::{AgentId, SessionId};

#[cfg(feature = "alloc")]
pub use agent::AgentContext;
```

---

## Commit Plan (11 commits)

| # | Message | Key change |
|---|---------|------------|
| 1 | `🔧 (aa-core): Add serde_json dev-dependency` | `serde_json = "1"` in `[dev-dependencies]` |
| 2 | `✨ (aa-core/identity): Add AgentId newtype over [u8; 16]` | New `src/identity.rs` with `AgentId` |
| 3 | `✨ (aa-core/identity): Add SessionId newtype over [u8; 16]` | `SessionId` added to `identity.rs` |
| 4 | `✨ (aa-core/identity): Add serde derives to AgentId and SessionId` | `cfg_attr` serde derives on both |
| 5 | `✅ (aa-core/identity): Add unit tests for AgentId and SessionId` | Round-trip, equality, `Copy` tests |
| 6 | `✨ (aa-core): Declare identity module and re-export AgentId, SessionId` | `pub mod identity` + `pub use` in `lib.rs` |
| 7 | `✨ (aa-core/agent): Add AgentContext struct gated on alloc` | New `src/agent.rs` with full struct |
| 8 | `✨ (aa-core/agent): Add AgentContext::now() constructor under std feature` | `#[cfg(all(alloc, std))] impl` block |
| 9 | `✅ (aa-core/agent): Add field access, clone, and equality tests` | Tests gated on `alloc` |
| 10 | `✅ (aa-core/agent): Add serde round-trip test` | Test gated on `alloc + serde`, uses `serde_json` |
| 11 | `✨ (aa-core): Declare agent module and re-export AgentContext under alloc` | `pub mod agent` + conditional `pub use` in `lib.rs` |

---

## Acceptance Criteria (from ticket)

- [ ] `AgentContext` struct defined with all required fields
- [ ] Newtype wrappers `AgentId` and `SessionId` implemented
- [ ] `no_std` compilation passes
- [ ] Unit tests verify field access, clone, and equality
- [ ] Serialization round-trip test passes (serde_json)

---

## Pattern for AAASM-23–25

Identity types (`AgentId`, `SessionId`) and `Timestamp` are now available in `aa-core`.
Future domain type tickets follow this template:

```rust
#[cfg(feature = "alloc")]
use alloc::{ /* heap types needed */ };
use crate::{identity::{AgentId, SessionId}, time::Timestamp};

#[cfg(feature = "alloc")]   // or just #[cfg] if heap-free
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MyDomainType { ... }
```

No future ticket needs to add new `Cargo.toml` entries — all feature wiring is already done.
