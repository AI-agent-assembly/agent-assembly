//! Error types for the policy simulation module.

use std::fmt;

/// Errors that can occur during policy simulation.
#[derive(Debug)]
pub enum SimulationError {
    /// The policy file could not be loaded or parsed.
    PolicyLoad(String),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyLoad(msg) => write!(f, "policy load error: {msg}"),
        }
    }
}

impl std::error::Error for SimulationError {}
