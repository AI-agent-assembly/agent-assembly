/// Pre-serialized JSON string passed at policy trait boundaries.
///
/// Callers serialize arguments before handing them to an evaluator;
/// evaluators deserialize lazily only if they need to inspect the payload.
/// This keeps the trait boundary free of any serde-json dependency.
#[cfg(feature = "alloc")]
pub type ArgsJson = alloc::string::String;

/// File access mode for `GovernanceAction::FileAccess`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileMode {
    Read,
    Write,
    Append,
    Delete,
}

/// Errors produced during policy loading or evaluation.
///
/// All variants are heap-free so `PolicyError` can be used in bare `no_std`
/// contexts that have no `alloc`.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyError {
    /// The supplied `PolicyDocument` is structurally invalid.
    InvalidDocument,
    /// The `GovernanceAction` variant is not recognized by this evaluator.
    UnknownAction,
    /// The evaluator encountered an internal error during evaluation.
    EvaluationFailed,
}

/// The decision recorded in a `PolicyRule`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyDecision {
    Allow,
    Deny,
    RequireApproval,
}

/// A single rule inside a `PolicyDocument`.
///
/// Gated on `alloc` because `action_pattern` is a `String`.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyRule {
    /// Glob-style pattern matched against the action name or path.
    pub action_pattern: alloc::string::String,
    /// Decision to apply when the pattern matches.
    pub decision: PolicyDecision,
}

/// Minimal policy document stub.
///
/// Full schema deferred to AAASM-105/AAASM-69. Sufficient for test evaluators
/// to implement `load_policy` and `validate_policy` without a real parser.
///
/// Gated on `alloc` because `name` and `rules` require heap allocation.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolicyDocument {
    /// Schema version number.
    pub version: u32,
    /// Human-readable policy name.
    pub name: alloc::string::String,
    /// Ordered list of rules evaluated top-to-bottom.
    pub rules: alloc::vec::Vec<PolicyRule>,
}

/// The outcome of a `PolicyEvaluator::evaluate` call.
///
/// Gated on `alloc` because `Deny::reason` carries a `String`.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyResult {
    /// The action is permitted.
    Allow,
    /// The action is denied; `reason` explains why.
    Deny { reason: alloc::string::String },
    /// Human approval is required within the given timeout.
    RequiresApproval { timeout_secs: u32 },
}

/// An agent action subject to governance evaluation.
///
/// Gated on `alloc` because all variants carry `String` fields.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GovernanceAction {
    /// Invocation of a named tool with pre-serialized JSON arguments.
    ToolCall {
        name: alloc::string::String,
        args: ArgsJson,
    },
    /// Read or write access to a file path.
    FileAccess {
        path: alloc::string::String,
        mode: FileMode,
    },
    /// Outbound network request.
    NetworkRequest {
        url: alloc::string::String,
        method: alloc::string::String,
    },
    /// Spawning an external process.
    ProcessExec { command: alloc::string::String },
}

/// Pluggable policy evaluation backend.
///
/// Implementors decide whether a given `GovernanceAction` is permitted for
/// a given `AgentContext`. The trait is object-safe: `dyn PolicyEvaluator`
/// is valid because no method has generic parameters or returns `Self`.
///
/// Gated on `alloc` because `GovernanceAction` and `PolicyDocument` require it.
#[cfg(feature = "alloc")]
pub trait PolicyEvaluator {
    /// Evaluate whether `action` is permitted for `ctx`.
    fn evaluate(&self, ctx: &crate::AgentContext, action: &GovernanceAction) -> PolicyResult;

    /// Load a policy document into this evaluator, replacing any prior policy.
    ///
    /// Requires `&mut self`, so callers holding `&dyn PolicyEvaluator` must
    /// upgrade to `&mut dyn PolicyEvaluator` before calling this method.
    fn load_policy(&mut self, policy: &PolicyDocument) -> Result<(), PolicyError>;

    /// Validate a policy document without applying it.
    ///
    /// Returns all validation errors found, or `Ok(())` if the document is valid.
    fn validate_policy(&self, policy: &PolicyDocument) -> Result<(), alloc::vec::Vec<PolicyError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_mode_clone_and_eq() {
        let m = FileMode::Read;
        assert_eq!(m.clone(), FileMode::Read);
        assert_ne!(FileMode::Write, FileMode::Delete);
    }

    #[test]
    fn file_mode_all_variants() {
        // Verify all variants are constructible and distinct.
        assert_ne!(FileMode::Read, FileMode::Write);
        assert_ne!(FileMode::Append, FileMode::Delete);
        assert_ne!(FileMode::Write, FileMode::Append);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn governance_action_tool_call() {
        let action = GovernanceAction::ToolCall {
            name: alloc::string::String::from("list_files"),
            args: alloc::string::String::from("{\"dir\":\"/tmp\"}"),
        };
        assert_eq!(action.clone(), action);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn governance_action_file_access() {
        let action = GovernanceAction::FileAccess {
            path: alloc::string::String::from("/etc/passwd"),
            mode: FileMode::Read,
        };
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn governance_action_network_request() {
        let action = GovernanceAction::NetworkRequest {
            url: alloc::string::String::from("https://example.com"),
            method: alloc::string::String::from("GET"),
        };
        assert_eq!(action.clone(), action);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn governance_action_spawn() {
        let action = GovernanceAction::ProcessExec {
            command: alloc::string::String::from("ls -la"),
        };
        assert_eq!(action.clone(), action);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_result_allow() {
        assert_eq!(PolicyResult::Allow, PolicyResult::Allow);
        assert_eq!(PolicyResult::Allow.clone(), PolicyResult::Allow);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_result_deny_reason() {
        let r = PolicyResult::Deny {
            reason: alloc::string::String::from("blocked"),
        };
        if let PolicyResult::Deny { reason } = &r {
            assert_eq!(reason, "blocked");
        } else {
            panic!("expected Deny");
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_result_requires_approval() {
        let r = PolicyResult::RequiresApproval { timeout_secs: 30 };
        if let PolicyResult::RequiresApproval { timeout_secs } = r {
            assert_eq!(timeout_secs, 30);
        } else {
            panic!("expected RequiresApproval");
        }
    }

    #[test]
    fn policy_error_variants() {
        assert_eq!(PolicyError::InvalidDocument, PolicyError::InvalidDocument);
        assert_ne!(PolicyError::UnknownAction, PolicyError::EvaluationFailed);
    }

    #[test]
    fn policy_decision_variants() {
        assert_eq!(PolicyDecision::Allow, PolicyDecision::Allow);
        assert_ne!(PolicyDecision::Deny, PolicyDecision::RequireApproval);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_rule_field_access_clone_eq() {
        let rule = PolicyRule {
            action_pattern: alloc::string::String::from("tool_call/*"),
            decision: PolicyDecision::Deny,
        };
        let cloned = rule.clone();
        assert_eq!(rule, cloned);
        assert_eq!(rule.action_pattern, "tool_call/*");
        assert_eq!(rule.decision, PolicyDecision::Deny);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_document_field_access_clone_eq() {
        let doc = PolicyDocument {
            version: 1,
            name: alloc::string::String::from("test-policy"),
            rules: alloc::vec![PolicyRule {
                action_pattern: alloc::string::String::from("*"),
                decision: PolicyDecision::Allow,
            }],
        };
        let cloned = doc.clone();
        assert_eq!(doc, cloned);
        assert_eq!(doc.version, 1);
        assert_eq!(doc.name, "test-policy");
        assert_eq!(doc.rules.len(), 1);
        assert_eq!(doc.rules[0].decision, PolicyDecision::Allow);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn policy_result_cross_variant_inequality() {
        assert_ne!(
            PolicyResult::Allow,
            PolicyResult::Deny {
                reason: alloc::string::String::from("x")
            }
        );
        assert_ne!(
            PolicyResult::Deny {
                reason: alloc::string::String::from("x")
            },
            PolicyResult::RequiresApproval { timeout_secs: 10 }
        );
    }
}
