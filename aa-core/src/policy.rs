/// Pre-serialized JSON string passed at policy trait boundaries.
///
/// Callers serialize arguments before handing them to an evaluator;
/// evaluators deserialize lazily only if they need to inspect the payload.
/// This keeps the trait boundary free of any serde-json dependency.
pub type ArgsJson = String;

/// File access mode for `GovernanceAction::FileAccess`.
#[derive(Debug, Clone, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    Append,
    Delete,
}
