# AAASM-69 — Policy YAML Parser & Typed Validator Design

**Date:** 2026-04-28  
**Ticket:** [AAASM-69](https://lightning-dust-mite.atlassian.net/browse/AAASM-69)  
**Epic:** [AAASM-8](https://lightning-dust-mite.atlassian.net/browse/AAASM-8) — Governance Gateway & Policy Engine  
**Status:** Approved

---

## Problem Statement

`aa-gateway` needs to load human-readable YAML policy files and convert them into validated, typed Rust structs that the evaluation engine can act on. The parser must produce structured errors with field-level detail and surface unknown keys as warnings rather than hard failures.

---

## Architectural Boundary Decision

`aa-core::policy::PolicyDocument` remains the **minimal, `no_std`-compatible stub** used at the `PolicyEvaluator` trait boundary (AAASM-23). It will not be extended with YAML-specific fields.

`aa-gateway` owns the **full, rich `PolicyDocument`** with all five policy sections. This is where `serde_yaml` lives — `aa-core` must stay `no_std` and cannot depend on a std-only YAML parser.

---

## Module Layout

```
aa-gateway/src/
  lib.rs                  ← pub mod policy
  policy/
    mod.rs                ← pub use of key public types
    error.rs              ← ValidationError, ValidationWarning
    raw.rs                ← RawPolicyDocument + Raw* section structs (serde targets)
    document.rs           ← PolicyDocument + validated section structs
    validator.rs          ← PolicyValidator::from_yaml()
```

---

## Dependencies

Added to `aa-gateway/Cargo.toml`:

| Crate | Version | Purpose |
|---|---|---|
| `serde` | `1` | Derive macros for `Deserialize` on Raw* structs |
| `serde_yaml` | `0.9` | YAML deserialization |
| `regex` | `1` | Compile-and-validate regex patterns in `data.sensitive_patterns` |

---

## Types

### `ValidationError` and `ValidationWarning` (`error.rs`)

```rust
pub struct ValidationError {
    /// Dot-notation field path, e.g. "budget.daily_limit_usd"
    pub field: String,
    /// Human-readable description of the constraint that was violated
    pub message: String,
    /// Best-effort line number from the YAML source (None if not determinable)
    pub line: Option<u32>,
}

pub struct ValidationWarning {
    /// The unknown key name (top-level or section-level)
    pub field: String,
    pub message: String,
}
```

### `RawPolicyDocument` (`raw.rs`)

Unvalidated serde deserialization target. All sections are `Option<T>` (absent sections are valid — they default to permissive). Unknown keys at the document level are captured via `#[serde(flatten)]` into a `HashMap<String, serde_yaml::Value>` and converted to `ValidationWarning` by the validator.

```rust
#[derive(Debug, Deserialize)]
pub struct RawPolicyDocument {
    pub network:  Option<RawNetworkPolicy>,
    pub tools:    Option<HashMap<String, RawToolPolicy>>,
    pub data:     Option<RawDataPolicy>,
    pub budget:   Option<RawBudgetPolicy>,
    pub schedule: Option<RawSchedulePolicy>,
    #[serde(flatten)]
    pub unknown:  HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawNetworkPolicy {
    pub allowlist: Option<Vec<String>>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawToolPolicy {
    pub allow:               Option<bool>,
    pub limit_per_hour:      Option<u32>,
    pub requires_approval_if: Option<String>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawDataPolicy {
    pub sensitive_patterns: Option<Vec<String>>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawBudgetPolicy {
    pub daily_limit_usd: Option<f64>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawSchedulePolicy {
    pub active_hours: Option<RawActiveHours>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawActiveHours {
    pub start:    Option<String>,
    pub end:      Option<String>,
    pub timezone: Option<String>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

### `PolicyDocument` (`document.rs`)

Fully validated output. All sections present (defaulting to empty/permissive when absent in YAML).

```rust
pub struct PolicyDocument {
    pub network:  NetworkPolicy,
    pub tools:    HashMap<String, ToolPolicy>,
    pub data:     DataPolicy,
    pub budget:   BudgetPolicy,
    pub schedule: SchedulePolicy,
}

pub struct NetworkPolicy   { pub allowlist: Vec<String> }
pub struct ToolPolicy      { pub allow: bool, pub limit_per_hour: Option<u32>, pub requires_approval_if: Option<String> }
pub struct DataPolicy      { pub sensitive_patterns: Vec<String> }
pub struct BudgetPolicy    { pub daily_limit_usd: Option<f64> }
pub struct SchedulePolicy  { pub active_hours: Option<ActiveHours> }
pub struct ActiveHours     { pub start: String, pub end: String, pub timezone: String }
```

### `PolicyValidator` (`validator.rs`)

```rust
pub struct PolicyValidator;

impl PolicyValidator {
    /// Parse YAML source and return a validated PolicyDocument.
    ///
    /// Returns Ok((doc, warnings)) on success — warnings are non-fatal.
    /// Returns Err(errors) if any field-level constraint is violated.
    pub fn from_yaml(src: &str)
        -> Result<(PolicyDocument, Vec<ValidationWarning>), Vec<ValidationError>>;
}
```

The method:
1. Calls `serde_yaml::from_str::<RawPolicyDocument>(src)` — malformed YAML returns a single `ValidationError` with the serde_yaml error message and best-effort line number.
2. Calls `Self::validate(raw)` which:
   - Collects `ValidationWarning` for every key in any `unknown` remainder map.
   - Validates each field per the rules below.
   - Returns `Err(errors)` if `errors` is non-empty, otherwise `Ok((doc, warnings))`.

---

## Validation Rules

| Field path | Constraint | Error on violation |
|---|---|---|
| `network.allowlist[i]` | non-empty string | `ValidationError` |
| `tools.<name>.limit_per_hour` | `>= 1` if present | `ValidationError` |
| `tools.<name>.requires_approval_if` | non-empty if present | `ValidationError` |
| `data.sensitive_patterns[i]` | valid `regex::Regex` | `ValidationError` |
| `budget.daily_limit_usd` | `> 0.0` if present | `ValidationError` |
| `schedule.active_hours.start` | matches `^([01]\d\|2[0-3]):[0-5]\d$` | `ValidationError` |
| `schedule.active_hours.end` | same pattern; must be after `start` | `ValidationError` |
| `schedule.active_hours.timezone` | non-empty | `ValidationError` |
| any unknown key at any level | emit `ValidationWarning` | (non-fatal) |

---

## Data Flow

```
&str (YAML source)
  │
  ▼
serde_yaml::from_str::<RawPolicyDocument>
  │  malformed → Vec<ValidationError> (single entry with line hint)
  ▼
PolicyValidator::validate(raw: RawPolicyDocument)
  │  field errors → Vec<ValidationError>
  │  unknown keys → Vec<ValidationWarning>
  ▼
Result<(PolicyDocument, Vec<ValidationWarning>), Vec<ValidationError>>
```

---

## Testing Strategy

All tests live in `validator.rs` as `#[cfg(test)]` inline tests.

| Test name | What it covers |
|---|---|
| `valid_full_policy` | All 5 sections present and valid → `Ok` with no warnings |
| `absent_sections_default_permissive` | Empty YAML `{}` → `Ok`, all defaults |
| `invalid_budget_zero` | `budget.daily_limit_usd: 0` → `ValidationError` on that field |
| `invalid_budget_negative` | Negative budget → `ValidationError` |
| `invalid_regex_pattern` | Bad regex in `data.sensitive_patterns` → `ValidationError` |
| `invalid_schedule_time_format` | `start: "9:00"` (missing leading zero) → `ValidationError` |
| `invalid_schedule_end_before_start` | `end < start` → `ValidationError` |
| `empty_allowlist_entry` | Empty string in `allowlist` → `ValidationError` |
| `tool_limit_zero` | `limit_per_hour: 0` → `ValidationError` |
| `unknown_top_level_key` | Extra key at root level → `ValidationWarning` |
| `unknown_nested_key` | Extra key inside `network` → `ValidationWarning` |
| `malformed_yaml` | Invalid YAML syntax → `Err` with line hint |

---

## Out of Scope for AAASM-69

- CEL expression parsing for `requires_approval_if` (expression stored as opaque `String`)
- `network.blocklist` (Epic AAASM-8 shows it but AAASM-69 requirements omit it — deferred)
- `data.can_access_pii`, `pii_must_not_leave` fields (deferred to policy evaluation tickets)
- Wiring `PolicyValidator` into the evaluation engine (AAASM-70)
