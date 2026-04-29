//! Control plane for Agent Assembly — policy enforcement and agent registry.
//!
//! The gateway is the central coordination point: it maintains the agent
//! registry, evaluates governance policies, routes enforcement decisions
//! back to proxies and SDK shims, and writes the audit trail.

pub mod anomaly;
pub mod budget;
pub mod engine;
pub mod policy;
pub mod server;
pub mod service;
pub mod simulation;

pub use engine::{EvaluationResult, PolicyEngine, PolicyLoadError};
pub use service::PolicyServiceImpl;
