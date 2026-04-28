//! Policy version history for aa-gateway.
//!
//! Tracks applied policy versions with timestamps and change attribution,
//! enabling rollback to any previous version.

pub mod config;
pub mod meta;
pub mod snapshot;

pub use config::HistoryConfig;
pub use meta::PolicyVersionMeta;
pub use snapshot::PolicySnapshot;
