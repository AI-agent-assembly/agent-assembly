//! Tokio async runtime wrapper and agent lifecycle management.
//!
//! This crate wraps `tokio` to provide a consistent async execution environment
//! for Agent Assembly components. It handles runtime initialization, shutdown
//! coordination, and lifecycle hooks.

pub mod config;
pub mod lifecycle;
pub mod runtime;
