//! Tokio async runtime wrapper and agent lifecycle management.
//!
//! This crate wraps `tokio` to provide a consistent async execution environment
//! for Agent Assembly components. It handles runtime initialization, shutdown
//! coordination, and lifecycle hooks.

pub mod config;
pub mod health;
pub mod ipc;
pub mod lifecycle;
pub mod pipeline;
pub mod policy;
pub mod runtime;

pub use runtime::run;
