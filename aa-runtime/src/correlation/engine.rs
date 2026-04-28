//! Orchestrator that ties together sliding window, PID lineage, and config.
//!
//! The [`CorrelationEngine`] is the main entry point for the causal correlation
//! subsystem. It is intentionally synchronous — the caller (the Tokio event
//! loop in aa-runtime) handles channel I/O; the engine handles pure logic.

use super::config::CorrelationConfig;
use super::event::CorrelationEvent;
use super::outcome::CorrelationOutcome;
use super::pid::PidLineage;
use super::window::SlidingWindow;

/// The causal correlation engine.
///
/// Ingests intent events (from LLM responses) and action events (from eBPF
/// kernel probes), stores them in a sliding time window, and produces
/// [`CorrelationOutcome`] results by matching intents to actions using PID
/// lineage and configurable time windows.
#[derive(Debug)]
pub struct CorrelationEngine {
    config: CorrelationConfig,
    window: SlidingWindow,
    lineage: PidLineage,
}

impl CorrelationEngine {
    /// Create a new correlation engine with the given configuration.
    pub fn new(config: CorrelationConfig) -> Self {
        let window = SlidingWindow::new(config.window_ms, config.max_window_size);
        Self {
            config,
            window,
            lineage: PidLineage::new(),
        }
    }

    /// Ingest a correlation event into the sliding window.
    ///
    /// This is a synchronous operation — no I/O, just an insertion into the
    /// in-memory window.
    pub fn ingest(&mut self, event: CorrelationEvent) {
        self.window.insert(event);
    }

    /// Run the correlation algorithm over the current window contents.
    ///
    /// Returns all correlation outcomes (matched, unexpected, intent-without-action)
    /// found in the current window state.
    pub fn correlate(&self) -> Vec<CorrelationOutcome> {
        todo!("implement PID lineage + keyword matching correlation algorithm")
    }

    /// Evict events older than the configured time window.
    ///
    /// Should be called periodically by the runtime at `config.eviction_interval_ms`.
    pub fn evict(&mut self, now_ms: u64) {
        self.window.evict(now_ms);
    }

    /// Register a PID parent-child relationship for lineage tracking.
    pub fn register_pid(&mut self, child_pid: u32, parent_pid: u32) {
        self.lineage.register(child_pid, parent_pid);
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &CorrelationConfig {
        &self.config
    }
}
