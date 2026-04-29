//! Anomaly detection engine for `aa-gateway`.
//!
//! Monitors agent behavior in real-time and detects deviations from
//! established baselines. The engine covers seven anomaly types defined
//! in the Governance Gateway epic (AAASM-8 AC #5).
//!
//! Entry point (future): `detector::AnomalyDetector::detect`.

pub mod types;
pub use types::{AnomalyEvent, AnomalyResponse, AnomalyType};

pub mod baseline;
pub mod detector;
pub mod responder;
