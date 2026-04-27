//! Core domain logic for Agent Assembly.
//!
//! This crate is `no_std` compatible and contains the foundational types,
//! traits, and pure logic shared across all other crates in the workspace.
//! It has no runtime or I/O dependencies.
//!
//! # Feature Flags
//!
//! - `std` (default): enables `std`-dependent convenience impls (e.g. `From<SystemTime>`)
//! - `alloc`: enables heap types (`String`, `Vec`, `BTreeMap`) in `no_std` environments
//! - `serde`: enables `Serialize`/`Deserialize` derives on all core types (added in AAASM-22–25)
//! - `test-utils`: exposes `PermitAllEvaluator` and `DenyAllEvaluator` for downstream test code

#![cfg_attr(not(feature = "std"), no_std)]

cfg_if::cfg_if! {
    if #[cfg(feature = "alloc")] {
        extern crate alloc;
    }
}

pub mod agent;
pub mod evaluators;
pub mod identity;
pub mod policy;
pub mod time;

pub use identity::{AgentId, SessionId};
pub use policy::{ArgsJson, FileMode, PolicyDecision, PolicyError};

#[cfg(feature = "alloc")]
pub use agent::AgentContext;

#[cfg(feature = "alloc")]
pub use policy::{GovernanceAction, PolicyDocument, PolicyEvaluator, PolicyResult, PolicyRule};

#[cfg(all(feature = "alloc", feature = "test-utils"))]
pub use evaluators::{DenyAllEvaluator, PermitAllEvaluator};
