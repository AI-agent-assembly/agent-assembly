//! Event aggregation pipeline — receives IpcFrames, enriches, batches, and fans out.

pub mod event;
pub mod metrics;

pub use event::{EnrichedEvent, EventSource};
pub use metrics::PipelineMetrics;
