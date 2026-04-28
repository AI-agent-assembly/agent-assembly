# AAASM-69 Policy YAML Parser & Typed Validator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a Policy YAML parser and typed validator in `aa-gateway` that converts human-readable YAML into a validated `PolicyDocument`, surfacing field-level `ValidationError`s and `ValidationWarning`s for unknown keys.

**Architecture:** Two-step pipeline — `serde_yaml` deserializes YAML into `RawPolicyDocument` (all fields `Option<T>`, unknown keys captured via `#[serde(flatten)]`), then `PolicyValidator::from_yaml()` walks the raw struct and enforces per-field constraints, producing either a valid `PolicyDocument` or a list of errors. `aa-core::policy::PolicyDocument` is NOT modified — it stays as the minimal `no_std` evaluator-boundary stub.

**Tech Stack:** Rust, `serde 1` + derive, `serde_yaml 0.9`, `regex 1`

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `aa-gateway/Cargo.toml` | Add serde, serde_yaml, regex deps |
| Modify | `aa-gateway/src/lib.rs` | Expose `pub mod policy` |
| Create | `aa-gateway/src/policy/mod.rs` | Re-export public types |
| Create | `aa-gateway/src/policy/error.rs` | `ValidationError`, `ValidationWarning` |
| Create | `aa-gateway/src/policy/raw.rs` | `RawPolicyDocument` + `Raw*` section structs (serde targets) |
| Create | `aa-gateway/src/policy/document.rs` | `PolicyDocument` + validated section structs |
| Create | `aa-gateway/src/policy/validator.rs` | `PolicyValidator::from_yaml()` + all validation logic + unit tests |

---

## Task 1: Add serde_yaml, serde, and regex dependencies

**Files:**
- Modify: `aa-gateway/Cargo.toml`

- [ ] **Step 1.1: Add the three dependencies**

Open `aa-gateway/Cargo.toml`. The `[dependencies]` section currently has:
```toml
[dependencies]
aa-core    = { path = "../aa-core" }
aa-proto   = { path = "../aa-proto" }
aa-runtime = { path = "../aa-runtime" }
tokio      = { version = "1", features = ["full"] }
```

Change it to:
```toml
[dependencies]
aa-core    = { path = "../aa-core" }
aa-proto   = { path = "../aa-proto" }
aa-runtime = { path = "../aa-runtime" }
tokio      = { version = "1", features = ["full"] }
regex      = "1"
serde      = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
```

- [ ] **Step 1.2: Verify the workspace compiles**

```bash
cd agent-assembly
cargo check -p aa-gateway
```

Expected: `Checking aa-gateway ...` with no errors. (Warnings about unused imports are fine at this stage.)

- [ ] **Step 1.3: Commit**

```bash
git add aa-gateway/Cargo.toml
git commit -m "⬆️ (aa-gateway): Add serde_yaml, serde derive, and regex dependencies"
```

---

## Task 2: Create the policy module skeleton

**Files:**
- Create: `aa-gateway/src/policy/mod.rs`
- Modify: `aa-gateway/src/lib.rs`

- [ ] **Step 2.1: Create the policy directory and empty mod.rs**

Create `aa-gateway/src/policy/mod.rs` with:
```rust
//! Policy YAML parser and validator for aa-gateway.
//!
//! Entry point: [`validator::PolicyValidator::from_yaml`].
```

- [ ] **Step 2.2: Wire the module into lib.rs**

`aa-gateway/src/lib.rs` currently contains only a module-level doc comment. Append:
```rust
//! Control plane for Agent Assembly — policy enforcement and agent registry.
//!
//! The gateway is the central coordination point: it maintains the agent
//! registry, evaluates governance policies, routes enforcement decisions
//! back to proxies and SDK shims, and writes the audit trail.

pub mod policy;
```

- [ ] **Step 2.3: Verify**

```bash
cargo check -p aa-gateway
```

Expected: no errors.

- [ ] **Step 2.4: Commit**

```bash
git add aa-gateway/src/policy/mod.rs aa-gateway/src/lib.rs
git commit -m "✨ (aa-gateway/policy): Add empty policy module"
```

---

## Task 3: Add ValidationError

**Files:**
- Create: `aa-gateway/src/policy/error.rs`
- Modify: `aa-gateway/src/policy/mod.rs`

- [ ] **Step 3.1: Write the failing test first**

Create `aa-gateway/src/policy/error.rs` with only the test module:
```rust
//! Validation error and warning types for policy YAML parsing.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_new_sets_field_and_message() {
        let e = ValidationError::new("budget.daily_limit_usd", "must be > 0");
        assert_eq!(e.field, "budget.daily_limit_usd");
        assert_eq!(e.message, "must be > 0");
        assert_eq!(e.line, None);
    }

    #[test]
    fn validation_error_with_line_sets_line() {
        let e = ValidationError::new("network.allowlist[0]", "must not be empty")
            .with_line(7);
        assert_eq!(e.line, Some(7));
    }
}
```

- [ ] **Step 3.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::error 2>&1 | head -20
```

Expected: compile error — `ValidationError` not found.

- [ ] **Step 3.3: Implement ValidationError**

Add the struct above the `#[cfg(test)]` block in `error.rs`:
```rust
/// An error produced during policy document validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Dot-notation field path, e.g. `"budget.daily_limit_usd"`.
    pub field: String,
    /// Human-readable description of the violated constraint.
    pub message: String,
    /// Best-effort line number from the YAML source (`None` when not determinable).
    pub line: Option<u32>,
}

impl ValidationError {
    /// Create a new error with no line information.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            line: None,
        }
    }

    /// Attach a best-effort line number.
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}
```

- [ ] **Step 3.4: Expose error module from mod.rs**

Add to `aa-gateway/src/policy/mod.rs`:
```rust
//! Policy YAML parser and validator for aa-gateway.
//!
//! Entry point: [`validator::PolicyValidator::from_yaml`].

pub mod error;
```

- [ ] **Step 3.5: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::error
```

Expected:
```
test policy::error::tests::validation_error_new_sets_field_and_message ... ok
test policy::error::tests::validation_error_with_line_sets_line ... ok
```

- [ ] **Step 3.6: Commit**

```bash
git add aa-gateway/src/policy/error.rs aa-gateway/src/policy/mod.rs
git commit -m "✨ (aa-gateway/policy): Add ValidationError struct"
```

---

## Task 4: Add ValidationWarning

**Files:**
- Modify: `aa-gateway/src/policy/error.rs`

- [ ] **Step 4.1: Write the failing test**

Append to the `tests` module in `error.rs`:
```rust
    #[test]
    fn validation_warning_unknown_key_formats_message() {
        let w = ValidationWarning::unknown_key("risk_tier");
        assert_eq!(w.field, "risk_tier");
        assert!(w.message.contains("risk_tier"));
    }

    #[test]
    fn validation_warning_unknown_key_nested_path() {
        let w = ValidationWarning::unknown_key("network.blocklist");
        assert_eq!(w.field, "network.blocklist");
    }
```

- [ ] **Step 4.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::error 2>&1 | head -10
```

Expected: compile error — `ValidationWarning` not found.

- [ ] **Step 4.3: Implement ValidationWarning**

Add after the `ValidationError` impl block in `error.rs`:
```rust
/// A non-fatal warning produced during policy document validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
    /// Dot-notation path of the unexpected key.
    pub field: String,
    /// Human-readable message.
    pub message: String,
}

impl ValidationWarning {
    /// Construct a warning for an unknown key at the given path.
    pub fn unknown_key(field: impl Into<String>) -> Self {
        let field = field.into();
        let message = format!("Unknown key '{}' will be ignored", field);
        Self { field, message }
    }
}
```

- [ ] **Step 4.4: Run — verify all error tests pass**

```bash
cargo test -p aa-gateway -- policy::error
```

Expected: 4 tests pass.

- [ ] **Step 4.5: Commit**

```bash
git add aa-gateway/src/policy/error.rs
git commit -m "✨ (aa-gateway/policy): Add ValidationWarning struct"
```

---

## Task 5: Add RawNetworkPolicy

**Files:**
- Create: `aa-gateway/src/policy/raw.rs`
- Modify: `aa-gateway/src/policy/mod.rs`

- [ ] **Step 5.1: Write the failing test**

Create `aa-gateway/src/policy/raw.rs` with only the test:
```rust
//! Unvalidated serde deserialization targets for policy YAML.

use std::collections::HashMap;
use serde::Deserialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_network_deserializes_allowlist() {
        let yaml = "allowlist:\n  - api.openai.com\n  - slack.com\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allowlist, Some(vec![
            "api.openai.com".to_string(),
            "slack.com".to_string(),
        ]));
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_network_captures_unknown_keys() {
        let yaml = "allowlist:\n  - api.openai.com\nblocklist:\n  - \"*\"\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.unknown.contains_key("blocklist"));
    }

    #[test]
    fn raw_network_absent_allowlist_is_none() {
        let yaml = "{}\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allowlist, None);
    }
}
```

- [ ] **Step 5.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw 2>&1 | head -10
```

Expected: compile error — `RawNetworkPolicy` not found.

- [ ] **Step 5.3: Implement RawNetworkPolicy**

Add above the `#[cfg(test)]` block:
```rust
/// Raw (unvalidated) deserialization target for the `network` policy section.
#[derive(Debug, Deserialize)]
pub struct RawNetworkPolicy {
    /// Domain glob patterns the agent may connect to.
    pub allowlist: Option<Vec<String>>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 5.4: Add raw module to mod.rs**

Append to `aa-gateway/src/policy/mod.rs`:
```rust
pub mod raw;
```

- [ ] **Step 5.5: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

Expected: 3 tests pass.

- [ ] **Step 5.6: Commit**

```bash
git add aa-gateway/src/policy/raw.rs aa-gateway/src/policy/mod.rs
git commit -m "✨ (aa-gateway/policy): Add RawNetworkPolicy serde target"
```

---

## Task 6: Add RawToolPolicy

**Files:**
- Modify: `aa-gateway/src/policy/raw.rs`

- [ ] **Step 6.1: Write the failing test**

Append to the `tests` module in `raw.rs`:
```rust
    #[test]
    fn raw_tool_deserializes_all_fields() {
        let yaml = "allow: true\nlimit_per_hour: 10\nrequires_approval_if: \"amount > 100\"\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allow, Some(true));
        assert_eq!(raw.limit_per_hour, Some(10));
        assert_eq!(raw.requires_approval_if, Some("amount > 100".to_string()));
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_tool_allow_false_captured() {
        let yaml = "allow: false\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allow, Some(false));
        assert_eq!(raw.limit_per_hour, None);
    }

    #[test]
    fn raw_tool_captures_unknown_key() {
        let yaml = "allow: true\nconstraint: \"read-only\"\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.unknown.contains_key("constraint"));
    }
```

- [ ] **Step 6.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw::tests::raw_tool 2>&1 | head -10
```

Expected: compile error — `RawToolPolicy` not found.

- [ ] **Step 6.3: Implement RawToolPolicy**

Add after `RawNetworkPolicy` in `raw.rs`:
```rust
/// Raw (unvalidated) deserialization target for a single entry in `tools`.
#[derive(Debug, Deserialize)]
pub struct RawToolPolicy {
    /// Whether this tool is permitted.
    pub allow: Option<bool>,
    /// Max calls per hour; `None` means unlimited.
    pub limit_per_hour: Option<u32>,
    /// CEL expression that triggers human-in-the-loop approval.
    pub requires_approval_if: Option<String>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 6.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

Expected: all existing + 3 new tests pass.

- [ ] **Step 6.5: Commit**

```bash
git add aa-gateway/src/policy/raw.rs
git commit -m "✨ (aa-gateway/policy): Add RawToolPolicy serde target"
```

---

## Task 7: Add RawDataPolicy

**Files:**
- Modify: `aa-gateway/src/policy/raw.rs`

- [ ] **Step 7.1: Write the failing test**

Append to the `tests` module:
```rust
    #[test]
    fn raw_data_deserializes_sensitive_patterns() {
        let yaml = "sensitive_patterns:\n  - \"sk-[a-zA-Z0-9]{48}\"\n  - \"\\\\b\\\\d{4}\\\\b\"\n";
        let raw: RawDataPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.sensitive_patterns.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn raw_data_absent_patterns_is_none() {
        let yaml = "{}\n";
        let raw: RawDataPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.sensitive_patterns, None);
    }
```

- [ ] **Step 7.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw::tests::raw_data 2>&1 | head -10
```

Expected: compile error.

- [ ] **Step 7.3: Implement RawDataPolicy**

Add after `RawToolPolicy` in `raw.rs`:
```rust
/// Raw (unvalidated) deserialization target for the `data` policy section.
#[derive(Debug, Deserialize)]
pub struct RawDataPolicy {
    /// Regex patterns for PII / credential detection.
    pub sensitive_patterns: Option<Vec<String>>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 7.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

- [ ] **Step 7.5: Commit**

```bash
git add aa-gateway/src/policy/raw.rs
git commit -m "✨ (aa-gateway/policy): Add RawDataPolicy serde target"
```

---

## Task 8: Add RawBudgetPolicy

**Files:**
- Modify: `aa-gateway/src/policy/raw.rs`

- [ ] **Step 8.1: Write the failing test**

```rust
    #[test]
    fn raw_budget_deserializes_daily_limit() {
        let yaml = "daily_limit_usd: 50.0\n";
        let raw: RawBudgetPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.daily_limit_usd, Some(50.0));
    }

    #[test]
    fn raw_budget_absent_limit_is_none() {
        let yaml = "{}\n";
        let raw: RawBudgetPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.daily_limit_usd, None);
    }
```

- [ ] **Step 8.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw::tests::raw_budget 2>&1 | head -10
```

- [ ] **Step 8.3: Implement RawBudgetPolicy**

Add after `RawDataPolicy` in `raw.rs`:
```rust
/// Raw (unvalidated) deserialization target for the `budget` policy section.
#[derive(Debug, Deserialize)]
pub struct RawBudgetPolicy {
    /// Maximum USD spend per calendar day; `None` means no limit.
    pub daily_limit_usd: Option<f64>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 8.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

- [ ] **Step 8.5: Commit**

```bash
git add aa-gateway/src/policy/raw.rs
git commit -m "✨ (aa-gateway/policy): Add RawBudgetPolicy serde target"
```

---

## Task 9: Add RawSchedulePolicy and RawActiveHours

**Files:**
- Modify: `aa-gateway/src/policy/raw.rs`

- [ ] **Step 9.1: Write the failing tests**

```rust
    #[test]
    fn raw_active_hours_deserializes_all_fields() {
        let yaml = "start: \"09:00\"\nend: \"18:00\"\ntimezone: \"Asia/Taipei\"\n";
        let raw: RawActiveHours = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.start, Some("09:00".to_string()));
        assert_eq!(raw.end, Some("18:00".to_string()));
        assert_eq!(raw.timezone, Some("Asia/Taipei".to_string()));
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_schedule_active_hours_absent_is_none() {
        let yaml = "{}\n";
        let raw: RawSchedulePolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.active_hours.is_none(), true);
    }
```

- [ ] **Step 9.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw::tests::raw_active 2>&1 | head -10
```

- [ ] **Step 9.3: Implement RawActiveHours and RawSchedulePolicy**

Add after `RawBudgetPolicy` in `raw.rs`:
```rust
/// Raw (unvalidated) deserialization target for `schedule.active_hours`.
#[derive(Debug, Deserialize)]
pub struct RawActiveHours {
    /// Window start in `HH:MM` 24-hour format.
    pub start: Option<String>,
    /// Window end in `HH:MM` 24-hour format.
    pub end: Option<String>,
    /// IANA timezone name (e.g. `"Asia/Taipei"`).
    pub timezone: Option<String>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

/// Raw (unvalidated) deserialization target for the `schedule` policy section.
#[derive(Debug, Deserialize)]
pub struct RawSchedulePolicy {
    /// Time window during which the agent is permitted to run.
    pub active_hours: Option<RawActiveHours>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 9.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

- [ ] **Step 9.5: Commit**

```bash
git add aa-gateway/src/policy/raw.rs
git commit -m "✨ (aa-gateway/policy): Add RawSchedulePolicy and RawActiveHours serde targets"
```

---

## Task 10: Add RawPolicyDocument

**Files:**
- Modify: `aa-gateway/src/policy/raw.rs`

- [ ] **Step 10.1: Write the failing test**

```rust
    #[test]
    fn raw_policy_document_deserializes_all_sections() {
        let yaml = r#"
network:
  allowlist:
    - api.openai.com
tools:
  web_search:
    allow: true
    limit_per_hour: 20
data:
  sensitive_patterns:
    - "sk-[a-zA-Z0-9]{48}"
budget:
  daily_limit_usd: 25.0
schedule:
  active_hours:
    start: "09:00"
    end: "18:00"
    timezone: "UTC"
"#;
        let raw: RawPolicyDocument = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.network.is_some());
        assert!(raw.tools.is_some());
        assert!(raw.data.is_some());
        assert!(raw.budget.is_some());
        assert!(raw.schedule.is_some());
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_policy_document_captures_unknown_top_level_key() {
        let yaml = "agent: support-agent\nversion: 2\n";
        let raw: RawPolicyDocument = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.unknown.contains_key("agent"));
        assert!(raw.unknown.contains_key("version"));
    }

    #[test]
    fn raw_policy_document_empty_yaml_is_valid() {
        let yaml = "{}\n";
        let raw: RawPolicyDocument = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.network.is_none());
        assert!(raw.tools.is_none());
        assert!(raw.unknown.is_empty());
    }
```

- [ ] **Step 10.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::raw::tests::raw_policy_document 2>&1 | head -10
```

- [ ] **Step 10.3: Implement RawPolicyDocument**

Add at the top of `raw.rs` (before `RawNetworkPolicy`, after the `use` statements):
```rust
/// Unvalidated top-level deserialization target for a policy YAML document.
///
/// All sections are optional — absent sections default to permissive during
/// validation. Unknown top-level keys are captured in `unknown` and converted
/// to [`crate::policy::error::ValidationWarning`]s.
#[derive(Debug, Deserialize)]
pub struct RawPolicyDocument {
    /// Network access control section.
    pub network: Option<RawNetworkPolicy>,
    /// Per-tool governance rules, keyed by tool name.
    pub tools: Option<HashMap<String, RawToolPolicy>>,
    /// Data sensitivity controls.
    pub data: Option<RawDataPolicy>,
    /// Budget spend controls.
    pub budget: Option<RawBudgetPolicy>,
    /// Time-based access controls.
    pub schedule: Option<RawSchedulePolicy>,
    /// Unknown top-level keys for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}
```

- [ ] **Step 10.4: Run — verify all raw tests pass**

```bash
cargo test -p aa-gateway -- policy::raw
```

Expected: all tests pass (11+ tests).

- [ ] **Step 10.5: Commit**

```bash
git add aa-gateway/src/policy/raw.rs
git commit -m "✨ (aa-gateway/policy): Add RawPolicyDocument top-level serde target"
```

---

## Task 11: Add validated document types

**Files:**
- Create: `aa-gateway/src/policy/document.rs`
- Modify: `aa-gateway/src/policy/mod.rs`

- [ ] **Step 11.1: Write the failing tests**

Create `aa-gateway/src/policy/document.rs` with:
```rust
//! Validated policy document types produced by [`super::validator::PolicyValidator`].

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_document_fields_accessible() {
        let doc = PolicyDocument {
            network: NetworkPolicy { allowlist: vec!["api.openai.com".to_string()] },
            tools: HashMap::new(),
            data: DataPolicy { sensitive_patterns: vec![] },
            budget: BudgetPolicy { daily_limit_usd: Some(50.0) },
            schedule: SchedulePolicy { active_hours: None },
        };
        assert_eq!(doc.network.allowlist[0], "api.openai.com");
        assert_eq!(doc.budget.daily_limit_usd, Some(50.0));
    }

    #[test]
    fn active_hours_fields_accessible() {
        let ah = ActiveHours {
            start: "09:00".to_string(),
            end: "18:00".to_string(),
            timezone: "UTC".to_string(),
        };
        assert_eq!(ah.start, "09:00");
    }
}
```

- [ ] **Step 11.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::document 2>&1 | head -10
```

- [ ] **Step 11.3: Implement all document types**

Add above the `#[cfg(test)]` block:
```rust
/// Fully validated policy document produced by [`super::validator::PolicyValidator`].
///
/// All sections are present — absent sections in the YAML source default to
/// empty/permissive values.
#[derive(Debug)]
pub struct PolicyDocument {
    /// Network access controls.
    pub network: NetworkPolicy,
    /// Per-tool governance rules, keyed by tool name.
    pub tools: HashMap<String, ToolPolicy>,
    /// Data sensitivity controls.
    pub data: DataPolicy,
    /// Budget spend controls.
    pub budget: BudgetPolicy,
    /// Time-based access controls.
    pub schedule: SchedulePolicy,
}

/// Validated network access control section.
#[derive(Debug)]
pub struct NetworkPolicy {
    /// Domain glob patterns the agent may connect to. Empty = no restrictions.
    pub allowlist: Vec<String>,
}

/// Validated governance rule for a single tool.
#[derive(Debug)]
pub struct ToolPolicy {
    /// Whether this tool is permitted to be called.
    pub allow: bool,
    /// Maximum calls per hour; `None` means unlimited.
    pub limit_per_hour: Option<u32>,
    /// CEL expression (stored as opaque string) that triggers human approval.
    pub requires_approval_if: Option<String>,
}

/// Validated data sensitivity control section.
#[derive(Debug)]
pub struct DataPolicy {
    /// Pre-compiled-validated regex pattern strings for PII detection.
    pub sensitive_patterns: Vec<String>,
}

/// Validated budget spend control section.
#[derive(Debug)]
pub struct BudgetPolicy {
    /// Maximum USD spend per calendar day; `None` means no limit.
    pub daily_limit_usd: Option<f64>,
}

/// Validated time-based access control section.
#[derive(Debug)]
pub struct SchedulePolicy {
    /// Active time window; `None` means always active.
    pub active_hours: Option<ActiveHours>,
}

/// Validated active hours window.
#[derive(Debug)]
pub struct ActiveHours {
    /// Window start in validated `HH:MM` 24-hour format.
    pub start: String,
    /// Window end in validated `HH:MM` 24-hour format.
    pub end: String,
    /// IANA timezone name.
    pub timezone: String,
}
```

- [ ] **Step 11.4: Add document module to mod.rs**

Add to `aa-gateway/src/policy/mod.rs`:
```rust
pub mod document;
```

- [ ] **Step 11.5: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::document
```

Expected: 2 tests pass.

- [ ] **Step 11.6: Commit**

```bash
git add aa-gateway/src/policy/document.rs aa-gateway/src/policy/mod.rs
git commit -m "✨ (aa-gateway/policy): Add PolicyDocument and validated section structs"
```

---

## Task 12: Add PolicyValidator skeleton with malformed YAML handling

**Files:**
- Create: `aa-gateway/src/policy/validator.rs`
- Modify: `aa-gateway/src/policy/mod.rs`

- [ ] **Step 12.1: Write the failing tests**

Create `aa-gateway/src/policy/validator.rs` with:
```rust
//! Policy YAML parser and validator.

use std::collections::HashMap;

use crate::policy::{
    document::{
        ActiveHours, BudgetPolicy, DataPolicy, NetworkPolicy, PolicyDocument, SchedulePolicy,
        ToolPolicy,
    },
    error::{ValidationError, ValidationWarning},
    raw::RawPolicyDocument,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_yaml_returns_error_with_message() {
        let yaml = "network:\n  allowlist: [\n";  // unclosed bracket
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "<document>");
        assert!(!errors[0].message.is_empty());
    }

    #[test]
    fn malformed_yaml_error_has_line_hint() {
        let yaml = "network:\n  allowlist: [\n";
        let result = PolicyValidator::from_yaml(yaml);
        let errors = result.unwrap_err();
        // serde_yaml 0.9 may or may not provide a line; just check it's plumbed
        // (Some or None is both acceptable — this tests the path compiles)
        let _ = errors[0].line;
    }
}
```

- [ ] **Step 12.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::validator 2>&1 | head -10
```

Expected: compile error — `PolicyValidator` not found.

- [ ] **Step 12.3: Implement PolicyValidator skeleton**

Add above the `#[cfg(test)]` block in `validator.rs`:
```rust
/// Parses and validates a policy YAML document.
pub struct PolicyValidator;

impl PolicyValidator {
    /// Parse a YAML string and return a validated [`PolicyDocument`].
    ///
    /// # Returns
    /// - `Ok((doc, warnings))` — document is valid; `warnings` lists non-fatal unknown keys.
    /// - `Err(errors)` — one or more field-level constraint violations.
    pub fn from_yaml(
        src: &str,
    ) -> Result<(PolicyDocument, Vec<ValidationWarning>), Vec<ValidationError>> {
        let raw: RawPolicyDocument = serde_yaml::from_str(src).map_err(|e| {
            vec![ValidationError {
                field: "<document>".to_string(),
                message: e.to_string(),
                line: e.location().map(|l| l.line() as u32),
            }]
        })?;
        Self::validate(raw)
    }

    fn validate(
        raw: RawPolicyDocument,
    ) -> Result<(PolicyDocument, Vec<ValidationWarning>), Vec<ValidationError>> {
        let mut errors: Vec<ValidationError> = Vec::new();
        let mut warnings: Vec<ValidationWarning> = Vec::new();

        let network = Self::validate_network(raw.network, &mut errors, &mut warnings);
        let tools = Self::validate_tools(raw.tools, &mut errors, &mut warnings);
        let data = Self::validate_data(raw.data, &mut errors, &mut warnings);
        let budget = Self::validate_budget(raw.budget, &mut errors);
        let schedule = Self::validate_schedule(raw.schedule, &mut errors, &mut warnings);

        // Unknown top-level keys
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(key));
        }

        if errors.is_empty() {
            Ok((PolicyDocument { network, tools, data, budget, schedule }, warnings))
        } else {
            Err(errors)
        }
    }

    fn validate_network(
        raw: Option<crate::policy::raw::RawNetworkPolicy>,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> NetworkPolicy {
        let raw = match raw {
            None => return NetworkPolicy { allowlist: vec![] },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("network.{}", key)));
        }
        NetworkPolicy { allowlist: raw.allowlist.unwrap_or_default() }
    }

    fn validate_tools(
        raw: Option<HashMap<String, crate::policy::raw::RawToolPolicy>>,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> HashMap<String, ToolPolicy> {
        let raw = match raw {
            None => return HashMap::new(),
            Some(r) => r,
        };
        let mut tools = HashMap::new();
        for (name, tool) in raw {
            for key in tool.unknown.keys() {
                warnings.push(ValidationWarning::unknown_key(&format!("tools.{}.{}", name, key)));
            }
            tools.insert(name, ToolPolicy {
                allow: tool.allow.unwrap_or(true),
                limit_per_hour: tool.limit_per_hour,
                requires_approval_if: tool.requires_approval_if,
            });
        }
        tools
    }

    fn validate_data(
        raw: Option<crate::policy::raw::RawDataPolicy>,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> DataPolicy {
        let raw = match raw {
            None => return DataPolicy { sensitive_patterns: vec![] },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("data.{}", key)));
        }
        DataPolicy { sensitive_patterns: raw.sensitive_patterns.unwrap_or_default() }
    }

    fn validate_budget(
        raw: Option<crate::policy::raw::RawBudgetPolicy>,
        _errors: &mut Vec<ValidationError>,
    ) -> BudgetPolicy {
        let raw = match raw {
            None => return BudgetPolicy { daily_limit_usd: None },
            Some(r) => r,
        };
        BudgetPolicy { daily_limit_usd: raw.daily_limit_usd }
    }

    fn validate_schedule(
        raw: Option<crate::policy::raw::RawSchedulePolicy>,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchedulePolicy {
        let raw = match raw {
            None => return SchedulePolicy { active_hours: None },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("schedule.{}", key)));
        }
        let active_hours = raw.active_hours.map(|ah| {
            for key in ah.unknown.keys() {
                warnings.push(ValidationWarning::unknown_key(
                    &format!("schedule.active_hours.{}", key),
                ));
            }
            ActiveHours {
                start: ah.start.unwrap_or_default(),
                end: ah.end.unwrap_or_default(),
                timezone: ah.timezone.unwrap_or_default(),
            }
        });
        SchedulePolicy { active_hours }
    }
}
```

- [ ] **Step 12.4: Add validator module to mod.rs**

Add to `aa-gateway/src/policy/mod.rs`:
```rust
pub mod validator;
```

- [ ] **Step 12.5: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: 2 tests pass.

- [ ] **Step 12.6: Commit**

```bash
git add aa-gateway/src/policy/validator.rs aa-gateway/src/policy/mod.rs
git commit -m "✨ (aa-gateway/policy): Add PolicyValidator skeleton with malformed YAML handling"
```

---

## Task 13: Implement unknown key warnings

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

The skeleton already collects unknown keys. This task adds tests to verify the behaviour.

- [ ] **Step 13.1: Write the failing tests**

Add to the `tests` module in `validator.rs`:
```rust
    #[test]
    fn unknown_top_level_key_produces_warning() {
        let yaml = "agent: support-agent\nversion: 2\n";
        let (_, warnings) = PolicyValidator::from_yaml(yaml).unwrap();
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.contains(&"agent"), "expected 'agent' warning, got {:?}", fields);
        assert!(fields.contains(&"version"), "expected 'version' warning, got {:?}", fields);
    }

    #[test]
    fn unknown_nested_key_in_network_produces_warning() {
        let yaml = "network:\n  allowlist:\n    - api.openai.com\n  blocklist:\n    - \"*\"\n";
        let (_, warnings) = PolicyValidator::from_yaml(yaml).unwrap();
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.contains(&"network.blocklist"),
            "expected 'network.blocklist' warning, got {:?}", fields);
    }

    #[test]
    fn unknown_key_in_tool_produces_warning() {
        let yaml = "tools:\n  query_db:\n    allow: true\n    constraint: \"read-only\"\n";
        let (_, warnings) = PolicyValidator::from_yaml(yaml).unwrap();
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.iter().any(|f| f.contains("constraint")),
            "expected constraint warning, got {:?}", fields);
    }

    #[test]
    fn no_unknown_keys_produces_no_warnings() {
        let yaml = "network:\n  allowlist:\n    - api.openai.com\n";
        let (_, warnings) = PolicyValidator::from_yaml(yaml).unwrap();
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }
```

- [ ] **Step 13.2: Run — verify tests pass (skeleton already handles this)**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all 6 tests pass. If any fail, check the `validate` method is forwarding `unknown` keys for each section.

- [ ] **Step 13.3: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✅ (aa-gateway/policy): Add tests for unknown key ValidationWarning collection"
```

---

## Task 14: Implement network section validation

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

- [ ] **Step 14.1: Write the failing test**

Add to the `tests` module:
```rust
    #[test]
    fn empty_string_in_allowlist_produces_error() {
        let yaml = "network:\n  allowlist:\n    - api.openai.com\n    - \"\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("allowlist")),
            "expected allowlist error, got {:?}", errors
        );
    }
```

- [ ] **Step 14.2: Run — verify it fails**

```bash
cargo test -p aa-gateway -- policy::validator::tests::empty_string_in_allowlist 2>&1
```

Expected: FAIL — no error is returned yet.

- [ ] **Step 14.3: Add allowlist validation to validate_network**

Replace the `validate_network` method body in `validator.rs`:
```rust
    fn validate_network(
        raw: Option<crate::policy::raw::RawNetworkPolicy>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> NetworkPolicy {
        let raw = match raw {
            None => return NetworkPolicy { allowlist: vec![] },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("network.{}", key)));
        }
        let allowlist = raw.allowlist.unwrap_or_default();
        for (i, entry) in allowlist.iter().enumerate() {
            if entry.is_empty() {
                errors.push(ValidationError::new(
                    format!("network.allowlist[{}]", i),
                    "allowlist entry must not be empty",
                ));
            }
        }
        NetworkPolicy { allowlist }
    }
```

- [ ] **Step 14.4: Run — verify test passes**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass.

- [ ] **Step 14.5: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✨ (aa-gateway/policy): Implement network allowlist empty-entry validation"
```

---

## Task 15: Implement tools section validation

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

- [ ] **Step 15.1: Write the failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn tool_limit_per_hour_zero_produces_error() {
        let yaml = "tools:\n  query_db:\n    allow: true\n    limit_per_hour: 0\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("limit_per_hour")),
            "expected limit_per_hour error, got {:?}", errors
        );
    }

    #[test]
    fn tool_requires_approval_if_empty_string_produces_error() {
        let yaml = "tools:\n  send_email:\n    allow: true\n    requires_approval_if: \"\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("requires_approval_if")),
            "expected requires_approval_if error, got {:?}", errors
        );
    }
```

- [ ] **Step 15.2: Run — verify they fail**

```bash
cargo test -p aa-gateway -- policy::validator::tests::tool_limit 2>&1
cargo test -p aa-gateway -- policy::validator::tests::tool_requires 2>&1
```

Expected: FAIL — no errors returned yet.

- [ ] **Step 15.3: Add validation to validate_tools**

Replace the `validate_tools` method body:
```rust
    fn validate_tools(
        raw: Option<HashMap<String, crate::policy::raw::RawToolPolicy>>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> HashMap<String, ToolPolicy> {
        let raw = match raw {
            None => return HashMap::new(),
            Some(r) => r,
        };
        let mut tools = HashMap::new();
        for (name, tool) in raw {
            for key in tool.unknown.keys() {
                warnings.push(ValidationWarning::unknown_key(
                    &format!("tools.{}.{}", name, key),
                ));
            }
            if let Some(limit) = tool.limit_per_hour {
                if limit < 1 {
                    errors.push(ValidationError::new(
                        format!("tools.{}.limit_per_hour", name),
                        "limit_per_hour must be >= 1",
                    ));
                }
            }
            if let Some(ref expr) = tool.requires_approval_if {
                if expr.is_empty() {
                    errors.push(ValidationError::new(
                        format!("tools.{}.requires_approval_if", name),
                        "requires_approval_if must not be empty when present",
                    ));
                }
            }
            tools.insert(name, ToolPolicy {
                allow: tool.allow.unwrap_or(true),
                limit_per_hour: tool.limit_per_hour,
                requires_approval_if: tool.requires_approval_if,
            });
        }
        tools
    }
```

- [ ] **Step 15.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass.

- [ ] **Step 15.5: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✨ (aa-gateway/policy): Implement tools limit_per_hour and requires_approval_if validation"
```

---

## Task 16: Implement data section regex validation

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

- [ ] **Step 16.1: Write the failing test**

Add to the `tests` module:
```rust
    #[test]
    fn invalid_regex_in_sensitive_patterns_produces_error() {
        let yaml = "data:\n  sensitive_patterns:\n    - \"sk-[valid]\"\n    - \"[invalid\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("sensitive_patterns")),
            "expected sensitive_patterns error, got {:?}", errors
        );
    }

    #[test]
    fn valid_regex_patterns_produce_no_error() {
        let yaml = "data:\n  sensitive_patterns:\n    - \"sk-[a-zA-Z0-9]{48}\"\n    - \"\\\\b\\\\d{16}\\\\b\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok(), "unexpected error: {:?}", result.unwrap_err());
    }
```

- [ ] **Step 16.2: Run — verify the invalid test fails**

```bash
cargo test -p aa-gateway -- policy::validator::tests::invalid_regex 2>&1
```

Expected: FAIL.

- [ ] **Step 16.3: Add regex validation to validate_data**

Add `use regex::Regex;` at the top of `validator.rs` (after the existing `use` statements).

Replace the `validate_data` method body:
```rust
    fn validate_data(
        raw: Option<crate::policy::raw::RawDataPolicy>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> DataPolicy {
        let raw = match raw {
            None => return DataPolicy { sensitive_patterns: vec![] },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("data.{}", key)));
        }
        let patterns = raw.sensitive_patterns.unwrap_or_default();
        for (i, pattern) in patterns.iter().enumerate() {
            if let Err(e) = Regex::new(pattern) {
                errors.push(ValidationError::new(
                    format!("data.sensitive_patterns[{}]", i),
                    format!("invalid regex: {}", e),
                ));
            }
        }
        DataPolicy { sensitive_patterns: patterns }
    }
```

- [ ] **Step 16.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass.

- [ ] **Step 16.5: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✨ (aa-gateway/policy): Implement data sensitive_patterns regex validation"
```

---

## Task 17: Implement budget section validation

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

- [ ] **Step 17.1: Write the failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn budget_daily_limit_zero_produces_error() {
        let yaml = "budget:\n  daily_limit_usd: 0\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field == "budget.daily_limit_usd"),
            "expected budget.daily_limit_usd error, got {:?}", errors
        );
    }

    #[test]
    fn budget_daily_limit_negative_produces_error() {
        let yaml = "budget:\n  daily_limit_usd: -5.0\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn budget_absent_daily_limit_is_valid() {
        let yaml = "budget: {}\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok());
        let (doc, _) = result.unwrap();
        assert_eq!(doc.budget.daily_limit_usd, None);
    }
```

- [ ] **Step 17.2: Run — verify failing tests fail**

```bash
cargo test -p aa-gateway -- policy::validator::tests::budget 2>&1
```

Expected: `budget_daily_limit_zero_produces_error` and `budget_daily_limit_negative_produces_error` fail.

- [ ] **Step 17.3: Add validation to validate_budget**

Replace the `validate_budget` method body:
```rust
    fn validate_budget(
        raw: Option<crate::policy::raw::RawBudgetPolicy>,
        errors: &mut Vec<ValidationError>,
    ) -> BudgetPolicy {
        let raw = match raw {
            None => return BudgetPolicy { daily_limit_usd: None },
            Some(r) => r,
        };
        if let Some(limit) = raw.daily_limit_usd {
            if limit <= 0.0 {
                errors.push(ValidationError::new(
                    "budget.daily_limit_usd",
                    "daily_limit_usd must be > 0",
                ));
            }
        }
        BudgetPolicy { daily_limit_usd: raw.daily_limit_usd }
    }
```

- [ ] **Step 17.4: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass.

- [ ] **Step 17.5: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✨ (aa-gateway/policy): Implement budget daily_limit_usd > 0 validation"
```

---

## Task 18: Implement schedule section validation

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

- [ ] **Step 18.1: Write the failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn schedule_bad_time_format_produces_error() {
        let yaml = "schedule:\n  active_hours:\n    start: \"9:00\"\n    end: \"18:00\"\n    timezone: \"UTC\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("start")),
            "expected start field error, got {:?}", errors
        );
    }

    #[test]
    fn schedule_end_before_start_produces_error() {
        let yaml = "schedule:\n  active_hours:\n    start: \"18:00\"\n    end: \"09:00\"\n    timezone: \"UTC\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("active_hours")),
            "expected active_hours error, got {:?}", errors
        );
    }

    #[test]
    fn schedule_empty_timezone_produces_error() {
        let yaml = "schedule:\n  active_hours:\n    start: \"09:00\"\n    end: \"18:00\"\n    timezone: \"\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.field.contains("timezone")),
            "expected timezone error, got {:?}", errors
        );
    }

    #[test]
    fn schedule_valid_active_hours_produces_no_error() {
        let yaml = "schedule:\n  active_hours:\n    start: \"09:00\"\n    end: \"18:00\"\n    timezone: \"Asia/Taipei\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok(), "unexpected error: {:?}", result.unwrap_err());
        let (doc, _) = result.unwrap();
        let ah = doc.schedule.active_hours.unwrap();
        assert_eq!(ah.start, "09:00");
        assert_eq!(ah.timezone, "Asia/Taipei");
    }

    #[test]
    fn schedule_absent_produces_no_error() {
        let yaml = "{}\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok());
        let (doc, _) = result.unwrap();
        assert!(doc.schedule.active_hours.is_none());
    }
```

- [ ] **Step 18.2: Run — verify failing tests fail**

```bash
cargo test -p aa-gateway -- policy::validator::tests::schedule 2>&1
```

Expected: the three error-producing tests fail.

- [ ] **Step 18.3: Add a HH:MM validation helper**

Add above `validate_schedule` in `validator.rs`:
```rust
    /// Returns true if `t` matches `HH:MM` 24-hour format.
    fn is_valid_hhmm(t: &str) -> bool {
        if t.len() != 5 || t.as_bytes()[2] != b':' {
            return false;
        }
        let (h, m) = (&t[..2], &t[3..]);
        let hh: u8 = h.parse().unwrap_or(99);
        let mm: u8 = m.parse().unwrap_or(99);
        hh <= 23 && mm <= 59
    }
```

- [ ] **Step 18.4: Add full validation to validate_schedule**

Replace the `validate_schedule` method body:
```rust
    fn validate_schedule(
        raw: Option<crate::policy::raw::RawSchedulePolicy>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> SchedulePolicy {
        let raw = match raw {
            None => return SchedulePolicy { active_hours: None },
            Some(r) => r,
        };
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("schedule.{}", key)));
        }
        let active_hours = raw.active_hours.map(|ah| {
            for key in ah.unknown.keys() {
                warnings.push(ValidationWarning::unknown_key(
                    &format!("schedule.active_hours.{}", key),
                ));
            }
            let start = ah.start.unwrap_or_default();
            let end = ah.end.unwrap_or_default();
            let timezone = ah.timezone.unwrap_or_default();

            if !Self::is_valid_hhmm(&start) {
                errors.push(ValidationError::new(
                    "schedule.active_hours.start",
                    format!("'{}' is not a valid HH:MM time (e.g. '09:00')", start),
                ));
            }
            if !Self::is_valid_hhmm(&end) {
                errors.push(ValidationError::new(
                    "schedule.active_hours.end",
                    format!("'{}' is not a valid HH:MM time (e.g. '18:00')", end),
                ));
            }
            if Self::is_valid_hhmm(&start) && Self::is_valid_hhmm(&end) && end <= start {
                errors.push(ValidationError::new(
                    "schedule.active_hours",
                    format!("end '{}' must be after start '{}'", end, start),
                ));
            }
            if timezone.is_empty() {
                errors.push(ValidationError::new(
                    "schedule.active_hours.timezone",
                    "timezone must not be empty",
                ));
            }
            ActiveHours { start, end, timezone }
        });
        SchedulePolicy { active_hours }
    }
```

- [ ] **Step 18.5: Run — verify all tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass.

- [ ] **Step 18.6: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✨ (aa-gateway/policy): Implement schedule active_hours HH:MM and end>start validation"
```

---

## Task 19: Add full-policy integration tests

**Files:**
- Modify: `aa-gateway/src/policy/validator.rs`

These tests cover the acceptance criteria holistically.

- [ ] **Step 19.1: Write integration tests**

Add to the `tests` module:
```rust
    #[test]
    fn valid_full_policy_parses_without_errors() {
        let yaml = r#"
network:
  allowlist:
    - api.openai.com
    - slack.com
tools:
  query_db:
    allow: true
    limit_per_hour: 100
  delete_record:
    allow: false
  process_refund:
    allow: true
    limit_per_hour: 10
    requires_approval_if: "amount > 100"
data:
  sensitive_patterns:
    - "sk-[a-zA-Z0-9]{48}"
    - "\\b\\d{4}[- ]?\\d{4}[- ]?\\d{4}[- ]?\\d{4}\\b"
budget:
  daily_limit_usd: 50.0
schedule:
  active_hours:
    start: "09:00"
    end: "18:00"
    timezone: "Asia/Taipei"
"#;
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok(), "unexpected errors: {:?}", result.unwrap_err());
        let (doc, warnings) = result.unwrap();
        assert_eq!(doc.network.allowlist.len(), 2);
        assert_eq!(doc.tools.len(), 3);
        assert_eq!(doc.tools["delete_record"].allow, false);
        assert_eq!(doc.data.sensitive_patterns.len(), 2);
        assert_eq!(doc.budget.daily_limit_usd, Some(50.0));
        let ah = doc.schedule.active_hours.unwrap();
        assert_eq!(ah.timezone, "Asia/Taipei");
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
    }

    #[test]
    fn empty_document_is_valid_with_permissive_defaults() {
        let yaml = "{}\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok(), "unexpected errors: {:?}", result.unwrap_err());
        let (doc, warnings) = result.unwrap();
        assert!(doc.network.allowlist.is_empty());
        assert!(doc.tools.is_empty());
        assert!(doc.data.sensitive_patterns.is_empty());
        assert_eq!(doc.budget.daily_limit_usd, None);
        assert!(doc.schedule.active_hours.is_none());
        assert!(warnings.is_empty());
    }

    #[test]
    fn multiple_validation_errors_all_reported() {
        let yaml = r#"
budget:
  daily_limit_usd: -1.0
data:
  sensitive_patterns:
    - "[bad_regex"
network:
  allowlist:
    - ""
"#;
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 3,
            "expected at least 3 errors (budget + regex + allowlist), got {:?}", errors);
    }
```

- [ ] **Step 19.2: Run — verify tests pass**

```bash
cargo test -p aa-gateway -- policy::validator
```

Expected: all tests pass (20+ tests total).

- [ ] **Step 19.3: Run the full gateway test suite**

```bash
cargo test -p aa-gateway
```

Expected: all tests pass, no regressions.

- [ ] **Step 19.4: Run clippy**

```bash
cargo clippy -p aa-gateway -- -D warnings
```

Fix any warnings before committing.

- [ ] **Step 19.5: Commit**

```bash
git add aa-gateway/src/policy/validator.rs
git commit -m "✅ (aa-gateway/policy): Add full-policy integration tests and permissive-defaults test"
```

---

## Task 20: Wire public API surface in policy/mod.rs

**Files:**
- Modify: `aa-gateway/src/policy/mod.rs`

- [ ] **Step 20.1: Re-export key types**

Replace the content of `aa-gateway/src/policy/mod.rs` with:
```rust
//! Policy YAML parser and validator for aa-gateway.
//!
//! # Quick start
//!
//! ```rust
//! use aa_gateway::policy::validator::PolicyValidator;
//!
//! let yaml = include_str!("../../policy-examples/low-risk.yaml");
//! // Note: the policy-examples use an older format; pass a v1 schema YAML here.
//! ```
//!
//! Entry point: [`validator::PolicyValidator::from_yaml`].

pub mod document;
pub mod error;
pub mod raw;
pub mod validator;

pub use document::PolicyDocument;
pub use error::{ValidationError, ValidationWarning};
pub use validator::PolicyValidator;
```

- [ ] **Step 20.2: Verify the crate compiles cleanly**

```bash
cargo build -p aa-gateway
cargo clippy -p aa-gateway -- -D warnings
```

Expected: zero errors, zero warnings.

- [ ] **Step 20.3: Run the full test suite one final time**

```bash
cargo test -p aa-gateway
```

Expected: all tests pass.

- [ ] **Step 20.4: Commit**

```bash
git add aa-gateway/src/policy/mod.rs
git commit -m "♻️ (aa-gateway/policy): Re-export PolicyDocument, ValidationError, PolicyValidator from policy module"
```

---

## Summary

| Task | Commit message | Files |
|---|---|---|
| 1 | `⬆️ (aa-gateway): Add serde_yaml, serde derive, and regex dependencies` | Cargo.toml |
| 2 | `✨ (aa-gateway/policy): Add empty policy module` | lib.rs, policy/mod.rs |
| 3 | `✨ (aa-gateway/policy): Add ValidationError struct` | error.rs, mod.rs |
| 4 | `✨ (aa-gateway/policy): Add ValidationWarning struct` | error.rs |
| 5 | `✨ (aa-gateway/policy): Add RawNetworkPolicy serde target` | raw.rs, mod.rs |
| 6 | `✨ (aa-gateway/policy): Add RawToolPolicy serde target` | raw.rs |
| 7 | `✨ (aa-gateway/policy): Add RawDataPolicy serde target` | raw.rs |
| 8 | `✨ (aa-gateway/policy): Add RawBudgetPolicy serde target` | raw.rs |
| 9 | `✨ (aa-gateway/policy): Add RawSchedulePolicy and RawActiveHours serde targets` | raw.rs |
| 10 | `✨ (aa-gateway/policy): Add RawPolicyDocument top-level serde target` | raw.rs |
| 11 | `✨ (aa-gateway/policy): Add PolicyDocument and validated section structs` | document.rs, mod.rs |
| 12 | `✨ (aa-gateway/policy): Add PolicyValidator skeleton with malformed YAML handling` | validator.rs, mod.rs |
| 13 | `✅ (aa-gateway/policy): Add tests for unknown key ValidationWarning collection` | validator.rs |
| 14 | `✨ (aa-gateway/policy): Implement network allowlist empty-entry validation` | validator.rs |
| 15 | `✨ (aa-gateway/policy): Implement tools limit_per_hour and requires_approval_if validation` | validator.rs |
| 16 | `✨ (aa-gateway/policy): Implement data sensitive_patterns regex validation` | validator.rs |
| 17 | `✨ (aa-gateway/policy): Implement budget daily_limit_usd > 0 validation` | validator.rs |
| 18 | `✨ (aa-gateway/policy): Implement schedule active_hours HH:MM and end>start validation` | validator.rs |
| 19 | `✅ (aa-gateway/policy): Add full-policy integration tests and permissive-defaults test` | validator.rs |
| 20 | `♻️ (aa-gateway/policy): Re-export PolicyDocument, ValidationError, PolicyValidator from policy module` | mod.rs |
