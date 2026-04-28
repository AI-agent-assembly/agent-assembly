//! Dry-run policy evaluation engine.

use std::sync::Arc;

use crate::PolicyEngine;

use super::replay::SimulationEvent;
use super::report::{EventOutcome, SimulationReport};

/// A simulation engine that evaluates events against a policy without enforcing decisions.
///
/// Wraps a [`PolicyEngine`] in dry-run mode — reuses the full 7-step evaluation
/// pipeline (schedule, network, tool allow/deny, rate limit, approval condition,
/// data pattern scan, budget) but suppresses all side effects: no audit log writes,
/// no alert triggers, no approval queue entries.
pub struct SimulationEngine {
    /// The real policy engine whose evaluate() pipeline is reused in dry-run mode.
    engine: Arc<PolicyEngine>,
}

impl SimulationEngine {
    /// Create a new simulation engine wrapping the given policy engine.
    ///
    /// The engine is shared via `Arc` so callers can retain a reference to the
    /// same engine used by the live enforcement path.
    pub fn new(engine: Arc<PolicyEngine>) -> Self {
        Self { engine }
    }

    /// Returns a reference to the underlying policy engine.
    pub fn engine(&self) -> &PolicyEngine {
        &self.engine
    }

    /// Evaluate a single event against the loaded policy in dry-run mode.
    ///
    /// Returns the outcome without writing to the audit log or triggering alerts.
    pub fn simulate_event(&self, _index: usize, _event: &SimulationEvent) -> EventOutcome {
        todo!("AAASM-73: evaluate event against policy engine pipeline")
    }

    /// Run the simulation against a sequence of events, producing an aggregate report.
    pub fn run(&self, _events: &[SimulationEvent]) -> SimulationReport {
        todo!("AAASM-73: iterate events and collect into SimulationReport")
    }
}
