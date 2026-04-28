//! Budget tracking engine for `aa-gateway`.
//!
//! Entry point: [`tracker::BudgetTracker::record_usage`].

pub mod types;
pub use types::{BudgetAlert, BudgetState, BudgetStatus, Model, Provider};

pub mod pricing;
pub use pricing::{PricingEntry, PricingLoadError};
