//! eBPF-based kernel-level monitoring hooks for Agent Assembly.
//!
//! This crate provides kernel-space probes that intercept system calls and
//! network events originating from AI agent processes, feeding data into
//! the governance pipeline without requiring code changes in agents.
