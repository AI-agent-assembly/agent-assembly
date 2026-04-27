/// Pre-serialized JSON string passed at policy trait boundaries.
///
/// Callers serialize arguments before handing them to an evaluator;
/// evaluators deserialize lazily only if they need to inspect the payload.
/// This keeps the trait boundary free of any serde-json dependency.
pub type ArgsJson = String;

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

/// An agent action subject to governance evaluation.
///
/// Gated on `alloc` because all variants carry `String` fields.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GovernanceAction {
    /// Invocation of a named tool with pre-serialized JSON arguments.
    ToolCall { name: alloc::string::String, args: ArgsJson },
    /// Read or write access to a file path.
    FileAccess { path: alloc::string::String, mode: FileMode },
    /// Outbound network request.
    NetworkRequest { url: alloc::string::String, method: alloc::string::String },
    /// Spawning an external process.
    ProcessExec { command: alloc::string::String },
}
