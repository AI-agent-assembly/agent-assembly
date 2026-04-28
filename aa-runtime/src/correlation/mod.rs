//! Intent-Action Causal Correlation engine.
//!
//! Matches LLM response intents (captured via SDK or proxy) to kernel-level
//! actions (captured via eBPF) using PID lineage and a configurable time
//! window. Detects intent→action divergence and unauthorized escalation.
//!
//! Inspired by the AgentSight paper.

pub mod config;
pub mod engine;
pub mod event;
pub mod outcome;
pub mod pid;
pub mod window;

pub use config::CorrelationConfig;
pub use engine::CorrelationEngine;
pub use event::{ActionEvent, CorrelationEvent, IntentEvent};
pub use outcome::{CausalCorrelation, CorrelationOutcome};
pub use pid::PidLineage;
pub use window::SlidingWindow;
