//! Core domain logic for Agent Assembly.
//!
//! This crate is `no_std` compatible and contains the foundational types,
//! traits, and pure logic shared across all other crates in the workspace.
//! It has no runtime or I/O dependencies.

#![cfg_attr(not(feature = "std"), no_std)]
