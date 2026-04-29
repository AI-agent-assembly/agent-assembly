//! HTTP presentation layer for Agent Assembly.
//!
//! This crate exposes the gateway's capabilities over HTTP using `axum`.
//! OpenAPI documentation is generated at build time from route annotations
//! via `utoipa`. CI validates that `openapi/v1.yaml` stays in sync with
//! the generated spec — a drift failure blocks merge.

pub mod error;
pub mod events;
pub mod state;
