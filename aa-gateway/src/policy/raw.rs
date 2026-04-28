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
}
