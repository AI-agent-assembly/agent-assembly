# AAASM-23: `PolicyEvaluator` Trait — Design Spec

**Date:** 2026-04-27
**Ticket:** [AAASM-23](https://lightning-dust-mite.atlassian.net/browse/AAASM-23)
**Epic:** [AAASM-2](https://lightning-dust-mite.atlassian.net/browse/AAASM-2) — Runtime Core Foundations
**Sprint:** AAA Sprint 1 (ends 2026-04-30)
**Branch:** `v0.0.1/AAASM-23/feat/policy_evaluator_trait`

---

## Purpose

Define the `PolicyEvaluator` trait in `aa-core` — the abstraction that decouples
governance decisions from their enforcement mechanism, allowing multiple policy
backends (YAML, OPA, custom) to be swapped without changing the call site.
Alongside the trait, define all associated types: `GovernanceAction`, `PolicyResult`,
`PolicyError`, `PolicyDocument`, `PolicyRule`, `PolicyDecision`, `FileMode`,
and `ArgsJson`. Expose two test-only evaluators (`PermitAllEvaluator`,
`DenyAllEvaluator`) behind a `test-utils` Cargo feature.

---

## Scope

### In scope
- `aa-core/Cargo.toml` — add `test-utils = []` feature
- `aa-core/src/policy.rs` — new file: all public types and the `PolicyEvaluator` trait
- `aa-core/src/evaluators.rs` — new file: `PermitAllEvaluator` and `DenyAllEvaluator` (gated on `test-utils`)
- `aa-core/src/lib.rs` — module declarations and re-exports

### Out of scope
- No real policy parsing (YAML, OPA) — `PolicyDocument` is a minimal stub
- No changes to CI, other workspace crates, or tooling
- No full policy schema — full schema deferred to AAASM-105/AAASM-69

---

## Design Decisions

### `args` type: `pub type ArgsJson = String`

`serde_json::Value` was rejected because it pulls in `serde_json` as a
mandatory dependency, coupling the trait boundary to a specific crate.
`ArgsJson = String` keeps the boundary crate-free: callers pre-serialize,
evaluators deserialize lazily only if they need to inspect arguments.

### `GovernanceAction`: `FileAccess { path, mode: FileMode }` + `NetworkRequest { url, method }`

`FileAccess` uses a `FileMode` companion enum rather than splitting into
separate `ReadFile`/`WriteFile` variants. This allows evaluators to
match on mode without combinatorial variant explosion. `NetworkRequest`
carries `method: String` (not an enum) because HTTP method extensibility
(`PATCH`, custom methods) is not the concern of this layer.

### Module structure: `src/policy.rs` + `src/evaluators.rs` (Option B)

Two files rather than one. `evaluators.rs` is gated on `test-utils` so
downstream crates that depend on `aa-core` without `test-utils` never
compile the test doubles into production builds. `policy.rs` is always
compiled (subject to `alloc` gating of individual items).

### `PolicyDocument`: minimal stub

Full schema deferred to AAASM-105/AAASM-69. The stub is:
`{ version: u32, name: String, rules: Vec<PolicyRule> }` where
`PolicyRule { action_pattern: String, decision: PolicyDecision }`.
This is sufficient for `PermitAllEvaluator` and `DenyAllEvaluator` to
implement `load_policy` and `validate_policy` without a real parser.

### `PolicyEvaluator` alloc gating

The trait is gated on `#[cfg(feature = "alloc")]` because `GovernanceAction`
(a parameter type) requires `alloc` for `String` fields. There is no useful
heap-free signature for `evaluate`.

---

## File Changes

### `aa-core/Cargo.toml`

```toml
[features]
default    = ["std"]
std        = ["alloc"]
alloc      = []
serde      = ["dep:serde"]
test-utils = []          # new
```

### `aa-core/src/policy.rs` (new file)

```rust
// ArgsJson — pre-serialized JSON string at trait boundaries
pub type ArgsJson = String;

// FileMode — stack-only, no alloc gate
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileMode { Read, Write, Append, Delete }

// GovernanceAction — heap-dependent (String fields), alloc-gated
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GovernanceAction {
    ToolCall       { name: String, args: ArgsJson },
    FileAccess     { path: String, mode: FileMode },
    NetworkRequest { url: String, method: String },
    ProcessExec    { command: String },
}

// PolicyResult — heap-dependent (Deny.reason), alloc-gated
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyResult {
    Allow,
    Deny               { reason: String },
    RequiresApproval   { timeout_secs: u32 },
}

// PolicyError — heap-free variants only
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyError { InvalidDocument, UnknownAction, EvaluationFailed }

// PolicyDecision — heap-free (used inside PolicyRule)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyDecision { Allow, Deny, RequireApproval }

// PolicyRule — alloc-gated (action_pattern: String)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyRule {
    pub action_pattern: String,
    pub decision:       PolicyDecision,
}

// PolicyDocument — minimal stub, alloc-gated
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyDocument {
    pub version: u32,
    pub name:    String,
    pub rules:   alloc::vec::Vec<PolicyRule>,
}

// PolicyEvaluator — object-safe trait, alloc-gated
#[cfg(feature = "alloc")]
pub trait PolicyEvaluator {
    fn evaluate(&self, ctx: &crate::AgentContext, action: &GovernanceAction) -> PolicyResult;
    fn load_policy(&mut self, policy: &PolicyDocument) -> Result<(), PolicyError>;
    fn validate_policy(&self, policy: &PolicyDocument) -> Result<(), alloc::vec::Vec<PolicyError>>;
}
```

### `aa-core/src/evaluators.rs` (new file)

```rust
// Both types gated on alloc + test-utils
#[cfg(all(feature = "alloc", feature = "test-utils"))]
use crate::policy::{GovernanceAction, PolicyDocument, PolicyError, PolicyEvaluator, PolicyResult};
#[cfg(all(feature = "alloc", feature = "test-utils"))]
use crate::AgentContext;

#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub struct PermitAllEvaluator;

#[cfg(all(feature = "alloc", feature = "test-utils"))]
impl PolicyEvaluator for PermitAllEvaluator {
    fn evaluate(&self, _ctx: &AgentContext, _action: &GovernanceAction) -> PolicyResult {
        PolicyResult::Allow
    }
    fn load_policy(&mut self, _policy: &PolicyDocument) -> Result<(), PolicyError> { Ok(()) }
    fn validate_policy(&self, _policy: &PolicyDocument) -> Result<(), alloc::vec::Vec<PolicyError>> { Ok(()) }
}

#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub struct DenyAllEvaluator;

#[cfg(all(feature = "alloc", feature = "test-utils"))]
impl PolicyEvaluator for DenyAllEvaluator {
    fn evaluate(&self, _ctx: &AgentContext, _action: &GovernanceAction) -> PolicyResult {
        PolicyResult::Deny { reason: alloc::string::String::from("denied by DenyAllEvaluator") }
    }
    fn load_policy(&mut self, _policy: &PolicyDocument) -> Result<(), PolicyError> { Ok(()) }
    fn validate_policy(&self, _policy: &PolicyDocument) -> Result<(), alloc::vec::Vec<PolicyError>> { Ok(()) }
}
```

### `aa-core/src/lib.rs` additions

```rust
pub mod evaluators;
pub mod policy;

pub use policy::{ArgsJson, FileMode, PolicyDecision, PolicyError};
#[cfg(feature = "alloc")]
pub use policy::{GovernanceAction, PolicyDocument, PolicyEvaluator, PolicyResult, PolicyRule};
#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub use evaluators::{DenyAllEvaluator, PermitAllEvaluator};
```

---

## Commit Plan (14 commits)

| # | Message | Key change |
|---|---------|------------|
| 1 | `🔧 (aa-core): Add test-utils feature flag` | `test-utils = []` in `[features]` |
| 2 | `✨ (aa-core/policy): Add FileMode enum` | `FileMode { Read, Write, Append, Delete }` in new `src/policy.rs` |
| 3 | `✨ (aa-core/policy): Add ArgsJson type alias` | `pub type ArgsJson = String` |
| 4 | `✨ (aa-core/policy): Add GovernanceAction enum gated on alloc` | All four action variants |
| 5 | `✨ (aa-core/policy): Add serde derives to FileMode and GovernanceAction` | `cfg_attr` serde derives on both |
| 6 | `✨ (aa-core/policy): Add PolicyError enum` | Three heap-free error variants |
| 7 | `✨ (aa-core/policy): Add PolicyDecision, PolicyRule, PolicyDocument` | Minimal stub types, alloc-gated |
| 8 | `✨ (aa-core/policy): Add PolicyResult enum gated on alloc` | `Allow`, `Deny { reason }`, `RequiresApproval { timeout_secs }` |
| 9 | `✨ (aa-core/policy): Add PolicyEvaluator trait gated on alloc` | Three methods; object-safe |
| 10 | `✅ (aa-core/policy): Add unit tests for FileMode, GovernanceAction, PolicyResult` | Clone, equality, variant coverage |
| 11 | `✨ (aa-core/evaluators): Add PermitAllEvaluator under test-utils feature` | New `src/evaluators.rs`, always returns `Allow` |
| 12 | `✨ (aa-core/evaluators): Add DenyAllEvaluator under test-utils feature` | Always returns `Deny { reason }` |
| 13 | `✅ (aa-core/evaluators): Add tests for PermitAllEvaluator and DenyAllEvaluator` | Verify evaluate semantics for both |
| 14 | `✨ (aa-core): Declare policy and evaluators modules and re-export public API` | `pub mod policy/evaluators` + all `pub use` in `lib.rs` |

---

## Testing Strategy

All tests live in `#[cfg(test)]` modules within each source file, not in `tests/`.

- **`policy.rs` tests** (gated on `alloc`): clone and equality for `FileMode`,
  `GovernanceAction`, `PolicyResult`, `PolicyDecision`; variant construction for
  all four `GovernanceAction` variants; `Deny.reason` and `RequiresApproval.timeout_secs`
  field access.

- **`evaluators.rs` tests** (gated on `alloc + test-utils`): `PermitAllEvaluator`
  returns `PolicyResult::Allow` for every `GovernanceAction` variant;
  `DenyAllEvaluator` returns `PolicyResult::Deny { .. }` for every variant;
  both implement `dyn PolicyEvaluator` (object-safety compile test).

- **`no_std` CI target** (`thumbv7em`): `FileMode`, `PolicyError`, `PolicyDecision`
  must compile without `alloc`.

---

## Acceptance Criteria (from ticket)

- [ ] `PolicyEvaluator` trait defined and object-safe
- [ ] All associated types defined with complete documentation
- [ ] `PermitAllEvaluator` and `DenyAllEvaluator` implemented for testing
- [ ] Unit tests verify evaluation semantics for each `PolicyResult` variant
- [ ] Trait compiles in `no_std` mode
