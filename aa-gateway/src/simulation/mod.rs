//! Policy simulation engine for dry-run evaluation.
//!
//! Allows testing a new policy against historical audit logs or live traffic
//! without enforcing any decisions. Entry point: [`engine::SimulationEngine`].

pub mod engine;
pub mod error;
pub mod live;
pub mod replay;
pub mod report;

pub use engine::SimulationEngine;
pub use error::SimulationError;
pub use live::LiveSimulation;
pub use replay::{HistoricalReplay, SimulationEvent};
pub use report::{EventOutcome, SimulationReport};
