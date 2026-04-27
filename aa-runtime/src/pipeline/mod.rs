//! Event aggregation pipeline — receives IpcFrames, enriches, batches, and fans out.

pub mod event;

pub use event::{EnrichedEvent, EventSource};
