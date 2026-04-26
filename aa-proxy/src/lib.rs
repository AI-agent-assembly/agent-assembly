//! Sidecar traffic interception proxy for Agent Assembly.
//!
//! This crate implements the Layer 2 interception model: a sidecar proxy that
//! sits alongside each AI agent process, intercepting MCP/A2A/ACP traffic
//! and enforcing governance policies before forwarding requests.
