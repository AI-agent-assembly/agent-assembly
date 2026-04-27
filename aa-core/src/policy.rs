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
