//! Foundational types for the AI dev tool governance framework.
//!
//! These types are referenced by the `DevToolAdapter` trait and by every
//! per-tool adapter (Claude Code, Codex, GitHub Copilot, Windsurf Cascade).
//! They are intentionally light and free of runtime dependencies so that
//! adapters can be implemented in `no_std` contexts where applicable.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Governance level applied to a managed AI dev tool or agent.
///
/// Variants are ordered such that
/// `L0Discover < L1Observe < L2Enforce < L3Native`. The derived `Ord`
/// implementation enables policies to express "at-least-this-level" rules,
/// for example `governance_level >= L2Enforce`.
///
/// | Level | Capability |
/// | --- | --- |
/// | [`L0Discover`][Self::L0Discover] | eBPF / proxy detects unknown agents and their external behavior. |
/// | [`L1Observe`][Self::L1Observe] | Network, file, process, and MCP observability without enforcement. |
/// | [`L2Enforce`][Self::L2Enforce] | Allow / deny, approval, redaction, and budget enforcement. |
/// | [`L3Native`][Self::L3Native] | Full SDK-integrated governance with identity, lineage, and semantic context. |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GovernanceLevel {
    /// L0 — Discover. eBPF / proxy detects unknown agents and their
    /// external behavior; no policy enforcement is applied.
    L0Discover,
    /// L1 — Observe. Network, file, process, and MCP observability
    /// without enforcement.
    L1Observe,
    /// L2 — Enforce. Allow / deny, approval, redaction, and budget
    /// enforcement applied to the governed tool.
    L2Enforce,
    /// L3 — Native. Full SDK-integrated governance with identity,
    /// lineage, and semantic context awareness.
    L3Native,
}
