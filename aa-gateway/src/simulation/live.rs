//! Live traffic simulation — observe real agent events in dry-run mode.
//!
//! Subscribes to the event stream with a read-only view, evaluates each
//! event against a policy without enforcing decisions or producing side effects.

use std::time::Duration;

use super::engine::SimulationEngine;
use super::error::SimulationError;
use super::report::SimulationReport;

/// Observes live agent traffic and evaluates events against a policy in dry-run mode.
///
/// Unlike [`super::replay::HistoricalReplay`], which reads from a static JSONL file,
/// `LiveSimulation` subscribes to the real-time event stream and runs for a
/// configurable duration before producing a report.
pub struct LiveSimulation {
    /// The simulation engine used to evaluate each observed event.
    engine: SimulationEngine,
    /// How long to observe before stopping and producing the report.
    duration: Duration,
}

impl LiveSimulation {
    /// Create a new live simulation with the given engine and observation duration.
    pub fn new(engine: SimulationEngine, duration: Duration) -> Self {
        Self { engine, duration }
    }

    /// Returns the configured observation duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns a reference to the underlying simulation engine.
    pub fn engine(&self) -> &SimulationEngine {
        &self.engine
    }

    /// Run the live simulation, observing events for the configured duration.
    ///
    /// Subscribes to the event stream, evaluates each event against the loaded
    /// policy, and collects results into a [`SimulationReport`]. Does not enforce
    /// any decisions or produce side effects.
    pub async fn run(&self) -> Result<SimulationReport, SimulationError> {
        todo!("AAASM-73: subscribe to event stream and evaluate for configured duration")
    }
}
