//! Authorization scope levels for API operations.

use serde::{Deserialize, Serialize};

/// Authorization scope level for API operations.
///
/// Variants are ordered by privilege: `Read < Write < Admin`.
/// A caller with `Admin` scope satisfies any scope requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Read-only access to resources.
    Read,
    /// Read and write access (create, update, delete).
    Write,
    /// Full administrative access including agent kill.
    Admin,
}

impl Scope {
    /// Check whether the given set of scopes satisfies this required scope.
    ///
    /// Returns `true` if any scope in `granted` is >= `self` in the
    /// privilege ordering.
    pub fn is_satisfied_by(self, granted: &[Scope]) -> bool {
        granted.iter().any(|s| *s >= self)
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Read => write!(f, "read"),
            Scope::Write => write!(f, "write"),
            Scope::Admin => write!(f, "admin"),
        }
    }
}
