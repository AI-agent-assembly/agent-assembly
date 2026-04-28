//! Unvalidated serde deserialization targets for policy YAML.

use std::collections::HashMap;

use serde::Deserialize;

/// Raw (unvalidated) deserialization target for the `network` policy section.
#[derive(Debug, Deserialize)]
pub struct RawNetworkPolicy {
    /// Domain glob patterns the agent may connect to.
    pub allowlist: Option<Vec<String>>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

/// Raw (unvalidated) deserialization target for the `data` policy section.
#[derive(Debug, Deserialize)]
pub struct RawDataPolicy {
    /// Regex patterns for PII / credential detection.
    pub sensitive_patterns: Option<Vec<String>>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

/// Raw (unvalidated) deserialization target for a single entry in `tools`.
#[derive(Debug, Deserialize)]
pub struct RawToolPolicy {
    /// Whether this tool is permitted.
    pub allow: Option<bool>,
    /// Max calls per hour; `None` means unlimited.
    pub limit_per_hour: Option<u32>,
    /// CEL expression that triggers human-in-the-loop approval.
    pub requires_approval_if: Option<String>,
    /// Unknown keys captured for warning emission.
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_yaml::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RawNetworkPolicy ────────────────────────────────────────────────────

    #[test]
    fn raw_network_deserializes_allowlist() {
        let yaml = "allowlist:\n  - api.openai.com\n  - slack.com\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            raw.allowlist,
            Some(vec!["api.openai.com".to_string(), "slack.com".to_string()])
        );
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_network_captures_unknown_keys() {
        let yaml = "allowlist:\n  - api.openai.com\nblocklist:\n  - \"*\"\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.unknown.contains_key("blocklist"));
    }

    #[test]
    fn raw_network_absent_allowlist_is_none() {
        let yaml = "{}\n";
        let raw: RawNetworkPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allowlist, None);
    }

    // ── RawDataPolicy ───────────────────────────────────────────────────────

    #[test]
    fn raw_data_deserializes_sensitive_patterns() {
        let yaml = "sensitive_patterns:\n  - \"sk-[a-zA-Z0-9]{48}\"\n  - \"\\\\b\\\\d{4}\\\\b\"\n";
        let raw: RawDataPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.sensitive_patterns.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn raw_data_absent_patterns_is_none() {
        let yaml = "{}\n";
        let raw: RawDataPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.sensitive_patterns, None);
    }

    // ── RawToolPolicy ───────────────────────────────────────────────────────

    #[test]
    fn raw_tool_deserializes_all_fields() {
        let yaml = "allow: true\nlimit_per_hour: 10\nrequires_approval_if: \"amount > 100\"\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allow, Some(true));
        assert_eq!(raw.limit_per_hour, Some(10));
        assert_eq!(raw.requires_approval_if, Some("amount > 100".to_string()));
        assert!(raw.unknown.is_empty());
    }

    #[test]
    fn raw_tool_allow_false_captured() {
        let yaml = "allow: false\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.allow, Some(false));
        assert_eq!(raw.limit_per_hour, None);
    }

    #[test]
    fn raw_tool_captures_unknown_key() {
        let yaml = "allow: true\nconstraint: \"read-only\"\n";
        let raw: RawToolPolicy = serde_yaml::from_str(yaml).unwrap();
        assert!(raw.unknown.contains_key("constraint"));
    }
}
