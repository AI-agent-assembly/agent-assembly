//! Dev-tool governance — foundational types describing AI development tools
//! governed by Agent Assembly.
//!
//! This module establishes the data vocabulary that the [`DevToolAdapter`]
//! trait (added by AAASM-923) and every per-tool adapter (Claude Code, Codex,
//! GitHub Copilot, Windsurf Cascade, and SaaS coding agents) consume.
//!
//! Gated on the `std` feature because [`DevToolInfo::install_path`] is a
//! [`std::path::PathBuf`].
//!
//! [`DevToolAdapter`]: <not yet defined; introduced in AAASM-923>

use std::path::PathBuf;
use std::string::String;

// ---------------------------------------------------------------------------
// DevToolKind
// ---------------------------------------------------------------------------

/// Identifier for an AI development tool that Agent Assembly governs.
///
/// Built-in variants cover the four major commercial tools tracked by
/// Epic 14 (Dev Tool Governance). Use [`DevToolKind::Custom`] for any tool
/// integration shipped via the plugin extension point — the inner string
/// is a stable identifier supplied by the adapter author.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DevToolKind {
    /// Anthropic's Claude Code CLI / desktop / IDE tool family.
    ClaudeCode,
    /// OpenAI's Codex CLI.
    Codex,
    /// GitHub Copilot in agent mode (IDE-host bound).
    GitHubCopilot,
    /// Codeium's Windsurf Cascade IDE agent.
    WindsurfCascade,
    /// A user-supplied tool integration; the inner [`String`] is a stable
    /// identifier supplied by the adapter author.
    Custom(String),
}

// ---------------------------------------------------------------------------
// GovernanceLevel
// ---------------------------------------------------------------------------

/// Capability tier an Agent Assembly adapter can achieve for a given tool.
///
/// Levels form a strict total order:
/// `L0Discover < L1Observe < L2Enforce < L3Native`. A higher level always
/// subsumes the capabilities of every lower level, so policies and operator
/// UIs can compare levels directly with `>=`.
///
/// | Level | Capability                                                        |
/// |-------|-------------------------------------------------------------------|
/// | L0    | Discover — eBPF / proxy detects the tool exists.                  |
/// | L1    | Observe — network, file, process, MCP observability.              |
/// | L2    | Enforce — allow / deny, approval, redaction, budget enforcement.  |
/// | L3    | Native — full SDK-integrated identity, lineage, semantic context. |
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GovernanceLevel {
    /// Detection only — the eBPF / proxy layer sees the tool but cannot read its semantics.
    L0Discover = 0,
    /// Observability — network, file, process, and MCP events are surfaced.
    L1Observe = 1,
    /// Enforcement — policy decisions can allow, deny, redact, or budget actions.
    L2Enforce = 2,
    /// Native governance — the tool participates via SDK with identity, team tree, lineage.
    L3Native = 3,
}

// ---------------------------------------------------------------------------
// DevToolInfo
// ---------------------------------------------------------------------------

/// Detection-time description of an installed AI development tool.
///
/// Returned by an adapter's `detect` method when the tool is present on the
/// host. All fields are public because this struct is a plain data record;
/// adapters and the launcher inspect every field directly.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DevToolInfo {
    /// Which tool family this is.
    pub kind: DevToolKind,
    /// Reported tool version, if the adapter can resolve it.
    pub version: Option<String>,
    /// Absolute path to the tool's executable or installation root.
    pub install_path: PathBuf,
    /// Capability tier the adapter can achieve for this tool on this host.
    pub governance_level: GovernanceLevel,
    /// Whether this tool participates in the Model Context Protocol.
    pub supports_mcp: bool,
    /// Whether this tool exposes a managed-settings surface that
    /// Agent Assembly can write into.
    pub supports_managed_settings: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- GovernanceLevel ---

    #[test]
    fn governance_level_ordering_l0_to_l3() {
        assert!(GovernanceLevel::L0Discover < GovernanceLevel::L1Observe);
        assert!(GovernanceLevel::L1Observe < GovernanceLevel::L2Enforce);
        assert!(GovernanceLevel::L2Enforce < GovernanceLevel::L3Native);
        assert!(GovernanceLevel::L0Discover < GovernanceLevel::L3Native);
    }

    #[test]
    fn governance_level_discriminants_are_0_through_3() {
        assert_eq!(GovernanceLevel::L0Discover as u8, 0);
        assert_eq!(GovernanceLevel::L1Observe as u8, 1);
        assert_eq!(GovernanceLevel::L2Enforce as u8, 2);
        assert_eq!(GovernanceLevel::L3Native as u8, 3);
    }

    // --- DevToolKind ---

    #[test]
    fn devtool_kind_built_in_variants_are_distinct() {
        let variants = [
            DevToolKind::ClaudeCode,
            DevToolKind::Codex,
            DevToolKind::GitHubCopilot,
            DevToolKind::WindsurfCascade,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn devtool_kind_custom_carries_identifier() {
        let a = DevToolKind::Custom(String::from("internal-editor"));
        let b = DevToolKind::Custom(String::from("internal-editor"));
        let c = DevToolKind::Custom(String::from("other-editor"));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- Serde round-trips (gated on `serde` feature) ---

    #[cfg(feature = "serde")]
    #[test]
    fn devtool_kind_serde_round_trip_built_in() {
        let kinds = [
            DevToolKind::ClaudeCode,
            DevToolKind::Codex,
            DevToolKind::GitHubCopilot,
            DevToolKind::WindsurfCascade,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).expect("serialize");
            let parsed: DevToolKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, kind);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn devtool_kind_serde_round_trip_custom() {
        let kind = DevToolKind::Custom(String::from("internal-editor"));
        let json = serde_json::to_string(&kind).expect("serialize");
        let parsed: DevToolKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, kind);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn governance_level_serde_round_trip() {
        let levels = [
            GovernanceLevel::L0Discover,
            GovernanceLevel::L1Observe,
            GovernanceLevel::L2Enforce,
            GovernanceLevel::L3Native,
        ];
        for level in levels {
            let json = serde_json::to_string(&level).expect("serialize");
            let parsed: GovernanceLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, level);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn devtool_info_serde_round_trip() {
        let info = DevToolInfo {
            kind: DevToolKind::ClaudeCode,
            version: Some(String::from("1.2.3")),
            install_path: PathBuf::from("/usr/local/bin/claude"),
            governance_level: GovernanceLevel::L2Enforce,
            supports_mcp: true,
            supports_managed_settings: true,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let parsed: DevToolInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.kind, info.kind);
        assert_eq!(parsed.version, info.version);
        assert_eq!(parsed.install_path, info.install_path);
        assert_eq!(parsed.governance_level, info.governance_level);
        assert_eq!(parsed.supports_mcp, info.supports_mcp);
        assert_eq!(parsed.supports_managed_settings, info.supports_managed_settings);
    }
}
