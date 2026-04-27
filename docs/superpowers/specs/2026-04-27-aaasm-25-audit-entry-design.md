# AAASM-25 — Immutable `AuditEntry` with SHA-256 Hash Chain

**Ticket:** [AAASM-25](https://lightning-dust-mite.atlassian.net/browse/AAASM-25)
**Date:** 2026-04-27
**Status:** Approved

---

## Overview

Implement the `AuditEntry` type in `aa-core` — the foundational tamper-evident record for the
Agent Assembly audit trail. Immutability is enforced through Rust's ownership system (no
mutation methods). Each entry is content-addressed via a SHA-256 hash that covers all
tamper-meaningful fields, forming a hash chain where each entry commits to the hash of its
predecessor.

---

## Module Structure

New file: `aa-core/src/audit.rs`, gated on `#[cfg(feature = "alloc")]`.
Consistent with the existing `alloc`-tier types (`GovernanceAction`, `AgentContext`).

**Changes:**

| File | Change |
|---|---|
| `aa-core/src/audit.rs` | New module — all types live here |
| `aa-core/src/lib.rs` | Add `pub mod audit;` + re-exports under `#[cfg(feature = "alloc")]` |
| `aa-core/Cargo.toml` | Add `sha2` optional dep, activate under `alloc` feature |

---

## Types

### `AuditEventType`

```rust
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AuditEventType {
    ToolCallIntercepted   = 0,
    PolicyViolation       = 1,
    CredentialLeakBlocked = 2,
    ApprovalRequested     = 3,
    ApprovalGranted       = 4,
    ApprovalDenied        = 5,
    BudgetLimitApproached = 6,
    BudgetLimitExceeded   = 7,
}
```

`#[repr(u32)]` allows `event_type as u32` to produce the canonical 4-byte discriminant
for the hash input. `as_str() -> &'static str` supports `Display` and logging.

### `AuditEntry`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AuditEntry {
    // All fields private — access via getters only.
    seq:           u64,
    timestamp_ns:  u64,
    event_type:    AuditEventType,
    agent_id:      AgentId,
    session_id:    SessionId,
    payload:       alloc::string::String,
    previous_hash: [u8; 32],
    entry_hash:    [u8; 32],
}
```

---

## Constructor

```rust
impl AuditEntry {
    pub fn new(
        seq:           u64,
        timestamp_ns:  u64,
        event_type:    AuditEventType,
        agent_id:      AgentId,
        session_id:    SessionId,
        payload:       alloc::string::String,
        previous_hash: [u8; 32],
    ) -> Self
}
```

`timestamp_ns` is **caller-supplied** (nanoseconds since Unix epoch). This keeps the
constructor `no_std`-compatible — in `std` environments callers use
`Timestamp::from(SystemTime::now()).as_nanos()`. `entry_hash` is computed internally by
`compute_hash()` and is never a constructor parameter.

Returns `Self` directly — SHA-256 computation via `sha2` is infallible. No `AuditError`
type is needed for this ticket's scope.

---

## Canonical Hash Input

`entry_hash = SHA-256(bytes)` where `bytes` is the concatenation of:

```
seq.to_be_bytes()                  //  8 bytes, big-endian u64
timestamp_ns.to_be_bytes()         //  8 bytes, big-endian u64
(event_type as u32).to_be_bytes()  //  4 bytes, big-endian u32 (repr discriminant)
agent_id.as_bytes()                // 16 bytes ([u8; 16])
session_id.as_bytes()              // 16 bytes ([u8; 16])
previous_hash                      // 32 bytes ([u8; 32])
payload.as_bytes()                 // variable — UTF-8 bytes of pre-serialized string
```

**Fixed-width prefix: 84 bytes.** Field order is canonical and documented here.
Verifiers must use this exact order. `previous_hash = [0u8; 32]` for the genesis entry.

---

## `verify_integrity()`

```rust
pub fn verify_integrity(&self) -> bool {
    let expected = Self::compute_hash(
        self.seq, self.timestamp_ns, &self.event_type,
        &self.agent_id, &self.session_id, &self.previous_hash, &self.payload,
    );
    expected == self.entry_hash
}
```

Re-runs the same hash computation from stored fields. Returns `false` if any field has
been altered (including via `unsafe` code). Used by the acceptance criterion test that
simulates field tampering.

---

## `Display` Format

```
[seq=42 ts=1714222134000000000 agent=0102030405060708090a0b0c0d0e0f10 session=... event=ToolCallIntercepted]
```

`agent_id` and `session_id` are rendered as lowercase hex strings. `payload` is **not**
included in `Display` output — it may be arbitrarily large and callers use `payload()`
directly when they need it.

---

## `payload` Type

`alloc::string::String` — pre-serialized UTF-8. JSON in practice
(`serde_json::to_string(...)`). The `AuditEntry` does not inspect or validate the payload
format. Deterministic hashing is guaranteed by UTF-8's well-defined byte encoding.

---

## Dependency

```toml
sha2 = { version = "0.10", default-features = false, features = ["alloc"], optional = true }
```

Activated under the `alloc` feature:

```toml
alloc = ["dep:sha2"]
```

`sha2 0.10` is pure Rust, `no_std` + `alloc` compatible, and widely used in the Rust
ecosystem (same family as `sha2` used by `ring`, `rustls`, etc.).

---

## Feature Gating

| Feature | Activates |
|---|---|
| `alloc` | `audit` module, `sha2` dep, `AuditEntry`, `AuditEventType` |
| `serde` | Serialize/Deserialize derives on both types |

The `audit` module does **not** require `std`. The existing `no_std` CI targets
(`thumbv7em-none-eabihf`, `wasm32-unknown-unknown`) will verify this with `--features alloc`.

---

## Acceptance Criteria

- [ ] `AuditEntry` has no public mutation methods
- [ ] `verify_integrity()` returns `false` if any field is altered via `unsafe` code
- [ ] Constructor computes and stores SHA-256 hash over all 7 tamper-meaningful fields
- [ ] `no_std` compilation passes (`--no-default-features --features alloc`)
- [ ] Unit tests verify integrity check catches simulated tampering

---

## Commit Plan (12 commits)

| # | Emoji | Scope | Key change |
|---|---|---|---|
| 1 | ⬆️ | `aa-core/Cargo.toml` | Add `sha2 0.10` optional dep; activate under `alloc` feature |
| 2 | ✨ | `aa-core/audit` | Add `AuditEventType` — `#[repr(u32)]` enum, 8 variants, `as_str()`, serde derives |
| 3 | ✨ | `aa-core/audit` | Add `AuditEntry` struct — 8 private fields, 8 getter methods, serde derives |
| 4 | ✨ | `aa-core/audit` | Add private `compute_hash()` — 84-byte fixed prefix + payload into SHA-256 |
| 5 | ✨ | `aa-core/audit` | Add `AuditEntry::new()` — 7 caller params, calls `compute_hash()`, returns `Self` |
| 6 | ✨ | `aa-core/audit` | Add `verify_integrity() -> bool` — re-runs hash, compares to stored `entry_hash` |
| 7 | ✨ | `aa-core/audit` | Implement `Display` for `AuditEntry` — `[seq=N ts=T agent=HEX session=HEX event=Name]` |
| 8 | ✨ | `aa-core/lib.rs` | Wire `audit` module — `pub mod audit` + crate-root re-exports, both `alloc`-gated |
| 9 | ✅ | `aa-core/audit` | Tests for `AuditEventType` — `as_str()` all 8 variants, discriminants 0–7, all distinct |
| 10 | ✅ | `aa-core/audit` | Tests for `AuditEntry::new()` and getters — non-zero hash, all getters, genesis entry |
| 11 | ✅ | `aa-core/audit` | Tests for `verify_integrity()` — true when clean; false after `unsafe` mutation of `seq`, `payload`, `event_type`, `previous_hash` |
| 12 | ✅ | `aa-core/audit` | Tests for hash chain linkage and `Display` — chained entries, `seq` uniqueness, Display format |
