//! Policy rule types loaded from the policy volume mount.

use serde::Deserialize;

/// A single policy rule: a named set of action strings that are blocked.
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyRule {
    /// Human-readable rule name (used in violation log messages).
    pub name: String,
    /// Action strings that this rule blocks.
    /// Matched against `AuditEvent` action fields during pipeline evaluation.
    pub blocked_actions: Vec<String>,
}

/// The full set of policy rules loaded at runtime startup.
///
/// An empty `PolicyRules` (zero rules) means no enforcement — all events pass through normally.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolicyRules {
    /// The list of rules to evaluate against each event.
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

impl PolicyRules {
    /// Returns `true` if no rules are loaded (policy enforcement is disabled).
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_rules_is_empty() {
        let rules = PolicyRules::default();
        assert!(rules.is_empty());
        assert_eq!(rules.rules.len(), 0);
    }

    #[test]
    fn policy_rules_is_empty_false_when_rules_present() {
        let rules = PolicyRules {
            rules: vec![PolicyRule {
                name: "test-rule".to_string(),
                blocked_actions: vec!["dangerous_action".to_string()],
            }],
        };
        assert!(!rules.is_empty());
    }

    #[test]
    fn policy_rule_fields_are_accessible() {
        let rule = PolicyRule {
            name: "block-exfil".to_string(),
            blocked_actions: vec!["send_email".to_string(), "upload_file".to_string()],
        };
        assert_eq!(rule.name, "block-exfil");
        assert_eq!(rule.blocked_actions.len(), 2);
    }
}
