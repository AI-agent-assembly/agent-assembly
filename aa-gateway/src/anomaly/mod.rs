//! Anomaly detection engine for `aa-gateway`.
//!
//! Monitors agent behavior in real-time and detects deviations from
//! established baselines. The engine covers seven anomaly types defined
//! in the Governance Gateway epic (AAASM-8 AC #5).
//!
//! Entry point: [`AnomalyDetector::detect`](detector::AnomalyDetector::detect).

pub mod baseline;
pub mod detector;
pub mod responder;
pub mod types;

pub use baseline::AgentBaseline;
pub use detector::AnomalyDetector;
pub use responder::AnomalyResponder;
pub use types::{AnomalyConfig, AnomalyEvent, AnomalyResponse, AnomalyType};
