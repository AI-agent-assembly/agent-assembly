//! Foundational types for the AI dev tool governance framework.
//!
//! These types are referenced by the `DevToolAdapter` trait and by every
//! per-tool adapter (Claude Code, Codex, GitHub Copilot, Windsurf Cascade).
//! They are intentionally light and free of runtime dependencies so that
//! adapters can be implemented in `no_std` contexts where applicable.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "std")]
use std::path::PathBuf;

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GovernanceLevel {
    /// L0 — Discover. eBPF / proxy detects unknown agents and their
    /// external behavior; no policy enforcement is applied.
    ///
    /// This is also the [`Default`] for [`GovernanceLevel`]: any agent or
    /// rule that does not declare a level is treated as L0 (discover-only).
    #[default]
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

/// Concrete kind of AI dev tool being governed.
///
/// Concrete variants are matched against built-in `DevToolAdapter`
/// implementations. The [`Custom`][Self::Custom] variant lets out-of-tree
/// adapters identify themselves by name without requiring a code change
/// to this enum.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DevToolKind {
    /// Anthropic Claude Code (CLI).
    ClaudeCode,
    /// OpenAI Codex CLI.
    Codex,
    /// GitHub Copilot operating in agent mode.
    GitHubCopilot,
    /// Codeium Windsurf Cascade IDE agent.
    WindsurfCascade,
    /// Adapter-defined custom tool identified by an opaque name string.
    Custom(String),
}

/// Error type for [`DevToolAdapter`] method failures.
///
/// This is an intentional minimal stub introduced together with the
/// `DevToolAdapter` trait so the trait signatures resolve. AAASM-925 will
/// populate the concrete error variants (`ToolNotFound`,
/// `DetectionFailed`, `SettingsGenerationFailed`, etc.) and add a
/// `thiserror::Error` derive. Marked `#[non_exhaustive]` so that addition
/// is not a breaking change for downstream callers.
///
/// [`DevToolAdapter`]: <not yet defined; introduced in this same Subtask>
#[cfg(feature = "alloc")]
#[derive(Debug)]
#[non_exhaustive]
pub enum AdapterError {}

/// Static metadata describing a detected AI dev tool installation.
///
/// Returned by `DevToolAdapter::detect` and used to drive registry
/// decisions, managed-settings generation, and per-tool launch wiring.
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DevToolInfo {
    /// Concrete tool variant.
    pub kind: DevToolKind,
    /// Tool version string, if reported by the binary.
    pub version: Option<String>,
    /// Absolute path to the installed tool binary.
    pub install_path: PathBuf,
    /// Highest governance level this installation can operate at.
    pub governance_level: GovernanceLevel,
    /// Whether the tool exposes MCP server configuration we can govern.
    pub supports_mcp: bool,
    /// Whether the tool reads governance config from a managed-settings file.
    pub supports_managed_settings: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_level_orders_l0_through_l3() {
        assert!(GovernanceLevel::L0Discover < GovernanceLevel::L1Observe);
        assert!(GovernanceLevel::L1Observe < GovernanceLevel::L2Enforce);
        assert!(GovernanceLevel::L2Enforce < GovernanceLevel::L3Native);
        assert!(GovernanceLevel::L0Discover < GovernanceLevel::L3Native);
    }

    #[cfg(all(feature = "serde", feature = "alloc"))]
    #[test]
    fn dev_tool_kind_round_trips_via_serde_json() {
        let cases = [
            DevToolKind::ClaudeCode,
            DevToolKind::Codex,
            DevToolKind::GitHubCopilot,
            DevToolKind::WindsurfCascade,
            DevToolKind::Custom(String::from("MyEditor")),
        ];
        for original in cases {
            let json = serde_json::to_string(&original).expect("serialize");
            let restored: DevToolKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored, original);
        }
    }

    #[cfg(all(feature = "serde", feature = "std"))]
    #[test]
    fn dev_tool_info_round_trips_via_serde_json() {
        let original = DevToolInfo {
            kind: DevToolKind::ClaudeCode,
            version: Some(String::from("1.2.3")),
            install_path: PathBuf::from("/usr/local/bin/claude"),
            governance_level: GovernanceLevel::L2Enforce,
            supports_mcp: true,
            supports_managed_settings: false,
        };
        let json1 = serde_json::to_string(&original).expect("serialize");
        let restored: DevToolInfo = serde_json::from_str(&json1).expect("deserialize");
        let json2 = serde_json::to_string(&restored).expect("re-serialize");
        assert_eq!(json1, json2);
    }
}
