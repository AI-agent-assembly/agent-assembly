//! Budget tracking engine for `aa-gateway`.
//!
//! Entry point: [`tracker::BudgetTracker::record_usage`].

pub mod types;
pub use types::{BudgetState, BudgetStatus, Model, Provider};
