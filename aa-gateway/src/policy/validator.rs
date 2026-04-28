//! Policy YAML validator: deserializes raw YAML then validates into typed structs.

use std::collections::HashMap;

use crate::policy::{
    document::{
        ActiveHours, BudgetPolicy, DataPolicy, NetworkPolicy, PolicyDocument, SchedulePolicy,
        ToolPolicy,
    },
    error::{ValidationError, ValidationWarning},
    raw::RawPolicyDocument,
};

/// Result of a successful parse+validate pass.
#[derive(Debug)]
pub struct PolicyValidatorOutput {
    /// The fully-validated policy document.
    pub document: PolicyDocument,
    /// Non-fatal warnings (unknown keys, etc.).
    pub warnings: Vec<ValidationWarning>,
}

/// Parses and validates a policy YAML document.
pub struct PolicyValidator;

impl PolicyValidator {
    /// Parse `yaml_str`, validate every section, and return a typed
    /// [`PolicyDocument`] together with any [`ValidationWarning`]s.
    ///
    /// Returns `Err` with accumulated [`ValidationError`]s when at least one
    /// hard constraint is violated, or when the YAML cannot be parsed.
    pub fn from_yaml(
        yaml_str: &str,
    ) -> Result<PolicyValidatorOutput, Vec<ValidationError>> {
        // Step 1 — parse raw YAML
        let raw: RawPolicyDocument = serde_yaml::from_str(yaml_str).map_err(|e| {
            vec![ValidationError::new("(document)", format!("YAML parse error: {}", e))]
        })?;

        let mut errors: Vec<ValidationError> = Vec::new();
        let mut warnings: Vec<ValidationWarning> = Vec::new();

        // Step 2 — collect top-level unknown keys
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(key));
        }

        // Step 3 — validate each section
        let network = Self::validate_network(raw.network, &mut errors, &mut warnings);
        let schedule = Self::validate_schedule(raw.schedule, &mut errors, &mut warnings);
        let budget = Self::validate_budget(raw.budget, &mut errors);
        let data = Self::validate_data(raw.data, &mut errors);
        let tools = Self::validate_tools(raw.tools, &mut errors, &mut warnings);

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(PolicyValidatorOutput {
            document: PolicyDocument {
                version: raw.version,
                network,
                schedule,
                budget,
                data,
                tools,
            },
            warnings,
        })
    }

    // ── Section validators ──────────────────────────────────────────────────

    fn validate_network(
        raw: Option<crate::policy::raw::RawNetworkPolicy>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Option<NetworkPolicy> {
        let raw = raw?;

        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("network.{}", key)));
        }

        let allowlist = raw.allowlist.unwrap_or_default();
        for (i, entry) in allowlist.iter().enumerate() {
            if entry.trim().is_empty() {
                errors.push(ValidationError::new(
                    format!("network.allowlist[{}]", i),
                    "allowlist entry must not be empty",
                ));
            }
        }

        Some(NetworkPolicy { allowlist })
    }

    fn validate_schedule(
        raw: Option<crate::policy::raw::RawSchedulePolicy>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Option<SchedulePolicy> {
        let raw = raw?;

        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!("schedule.{}", key)));
        }

        let active_hours = raw
            .active_hours
            .map(|ah| Self::validate_active_hours(ah, errors, warnings))
            .flatten();

        Some(SchedulePolicy { active_hours })
    }

    fn validate_active_hours(
        raw: crate::policy::raw::RawActiveHours,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Option<ActiveHours> {
        for key in raw.unknown.keys() {
            warnings.push(ValidationWarning::unknown_key(&format!(
                "schedule.active_hours.{}",
                key
            )));
        }

        let start = match raw.start {
            Some(s) => {
                if !is_hhmm(&s) {
                    errors.push(ValidationError::new(
                        "schedule.active_hours.start",
                        "must be in HH:MM 24-hour format",
                    ));
                    return None;
                }
                s
            }
            None => {
                errors.push(ValidationError::new(
                    "schedule.active_hours.start",
                    "required when active_hours is present",
                ));
                return None;
            }
        };

        let end = match raw.end {
            Some(e) => {
                if !is_hhmm(&e) {
                    errors.push(ValidationError::new(
                        "schedule.active_hours.end",
                        "must be in HH:MM 24-hour format",
                    ));
                    return None;
                }
                e
            }
            None => {
                errors.push(ValidationError::new(
                    "schedule.active_hours.end",
                    "required when active_hours is present",
                ));
                return None;
            }
        };

        if start >= end {
            errors.push(ValidationError::new(
                "schedule.active_hours",
                "start must be earlier than end",
            ));
            return None;
        }

        let timezone = match raw.timezone {
            Some(tz) => tz,
            None => {
                errors.push(ValidationError::new(
                    "schedule.active_hours.timezone",
                    "required when active_hours is present",
                ));
                return None;
            }
        };

        Some(ActiveHours { start, end, timezone })
    }

    fn validate_budget(
        raw: Option<crate::policy::raw::RawBudgetPolicy>,
        errors: &mut Vec<ValidationError>,
    ) -> Option<BudgetPolicy> {
        let raw = raw?;

        if let Some(limit) = raw.daily_limit_usd {
            if limit <= 0.0 {
                errors.push(ValidationError::new(
                    "budget.daily_limit_usd",
                    "must be greater than 0",
                ));
            }
        }

        Some(BudgetPolicy {
            daily_limit_usd: raw.daily_limit_usd,
        })
    }

    fn validate_data(
        raw: Option<crate::policy::raw::RawDataPolicy>,
        errors: &mut Vec<ValidationError>,
    ) -> Option<DataPolicy> {
        let raw = raw?;

        let patterns = raw.sensitive_patterns.unwrap_or_default();
        for (i, pattern) in patterns.iter().enumerate() {
            if regex::Regex::new(pattern).is_err() {
                errors.push(ValidationError::new(
                    format!("data.sensitive_patterns[{}]", i),
                    format!("invalid regex: {}", pattern),
                ));
            }
        }

        Some(DataPolicy {
            sensitive_patterns: patterns,
        })
    }

    fn validate_tools(
        raw: Option<HashMap<String, crate::policy::raw::RawToolPolicy>>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> HashMap<String, ToolPolicy> {
        let raw = match raw {
            Some(m) => m,
            None => return HashMap::new(),
        };

        let mut tools = HashMap::new();
        for (name, rt) in raw {
            for key in rt.unknown.keys() {
                warnings.push(ValidationWarning::unknown_key(&format!(
                    "tools.{}.{}",
                    name, key
                )));
            }

            if let Some(expr) = &rt.requires_approval_if {
                if expr.trim().is_empty() {
                    errors.push(ValidationError::new(
                        format!("tools.{}.requires_approval_if", name),
                        "CEL expression must not be empty",
                    ));
                }
            }

            tools.insert(
                name,
                ToolPolicy {
                    allow: rt.allow.unwrap_or(true),
                    limit_per_hour: rt.limit_per_hour,
                    requires_approval_if: rt.requires_approval_if,
                },
            );
        }
        tools
    }
}

/// Returns `true` if `s` matches `HH:MM` with valid 24-hour values.
fn is_hhmm(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }
    match (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
        (Ok(h), Ok(m)) => h < 24 && m < 60 && parts[0].len() == 2 && parts[1].len() == 2,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unknown key warnings ────────────────────────────────────────────────

    #[test]
    fn top_level_unknown_key_produces_warning() {
        let yaml = "risk_tier: high\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        assert!(out.warnings.iter().any(|w| w.field == "risk_tier"));
    }

    #[test]
    fn network_unknown_key_produces_warning() {
        let yaml = "network:\n  allowlist:\n    - api.openai.com\n  blocklist:\n    - \"*\"\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        assert!(out.warnings.iter().any(|w| w.field == "network.blocklist"));
    }

    #[test]
    fn tool_unknown_key_produces_warning() {
        let yaml = "tools:\n  bash:\n    allow: true\n    constraint: read-only\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        assert!(out
            .warnings
            .iter()
            .any(|w| w.field == "tools.bash.constraint"));
    }

    // ── Network allowlist validation ────────────────────────────────────────

    #[test]
    fn network_empty_allowlist_entry_is_an_error() {
        let yaml = "network:\n  allowlist:\n    - \"\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "network.allowlist[0]"));
    }

    #[test]
    fn network_valid_allowlist_round_trips() {
        let yaml = "network:\n  allowlist:\n    - api.openai.com\n    - slack.com\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        let np = out.document.network.unwrap();
        assert_eq!(np.allowlist, vec!["api.openai.com", "slack.com"]);
    }

    // ── Tool validation ─────────────────────────────────────────────────────

    #[test]
    fn tool_empty_requires_approval_if_is_an_error() {
        let yaml = "tools:\n  bash:\n    allow: true\n    requires_approval_if: \"   \"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.field == "tools.bash.requires_approval_if"));
    }

    #[test]
    fn tool_allow_defaults_to_true_when_absent() {
        let yaml = "tools:\n  bash:\n    limit_per_hour: 5\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        assert!(out.document.tools["bash"].allow);
    }

    #[test]
    fn tool_limit_per_hour_round_trips() {
        let yaml = "tools:\n  bash:\n    allow: true\n    limit_per_hour: 10\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        assert_eq!(out.document.tools["bash"].limit_per_hour, Some(10));
    }

    // ── Data sensitive_patterns validation ─────────────────────────────────

    #[test]
    fn data_invalid_regex_pattern_is_an_error() {
        let yaml = "data:\n  sensitive_patterns:\n    - \"[unclosed\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "data.sensitive_patterns[0]"));
    }

    #[test]
    fn data_valid_regex_patterns_round_trip() {
        let yaml = "data:\n  sensitive_patterns:\n    - \"sk-[a-zA-Z0-9]{48}\"\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        let dp = out.document.data.unwrap();
        assert_eq!(dp.sensitive_patterns.len(), 1);
    }

    // ── Budget validation ───────────────────────────────────────────────────

    #[test]
    fn budget_zero_daily_limit_is_an_error() {
        let yaml = "budget:\n  daily_limit_usd: 0.0\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "budget.daily_limit_usd"));
    }

    #[test]
    fn budget_negative_daily_limit_is_an_error() {
        let yaml = "budget:\n  daily_limit_usd: -1.0\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "budget.daily_limit_usd"));
    }

    #[test]
    fn budget_valid_daily_limit_round_trips() {
        let yaml = "budget:\n  daily_limit_usd: 50.0\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        let bp = out.document.budget.unwrap();
        assert_eq!(bp.daily_limit_usd, Some(50.0));
    }

    // ── Schedule active_hours validation ───────────────────────────────────

    #[test]
    fn schedule_invalid_start_format_is_an_error() {
        let yaml =
            "schedule:\n  active_hours:\n    start: \"9:00\"\n    end: \"18:00\"\n    timezone: \"UTC\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.field == "schedule.active_hours.start"));
    }

    #[test]
    fn schedule_end_not_after_start_is_an_error() {
        let yaml =
            "schedule:\n  active_hours:\n    start: \"18:00\"\n    end: \"09:00\"\n    timezone: \"UTC\"\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "schedule.active_hours"));
    }

    #[test]
    fn schedule_valid_active_hours_round_trips() {
        let yaml = "schedule:\n  active_hours:\n    start: \"09:00\"\n    end: \"18:00\"\n    timezone: \"Asia/Taipei\"\n";
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        let sp = out.document.schedule.unwrap();
        let ah = sp.active_hours.unwrap();
        assert_eq!(ah.start, "09:00");
        assert_eq!(ah.end, "18:00");
        assert_eq!(ah.timezone, "Asia/Taipei");
    }

    // ── Full-policy integration ─────────────────────────────────────────────

    #[test]
    fn full_policy_document_validates_successfully() {
        let yaml = r#"
version: "1.0"
network:
  allowlist:
    - api.openai.com
    - slack.com
schedule:
  active_hours:
    start: "09:00"
    end: "18:00"
    timezone: "Asia/Taipei"
budget:
  daily_limit_usd: 25.0
data:
  sensitive_patterns:
    - "sk-[a-zA-Z0-9]{48}"
tools:
  bash:
    allow: true
    limit_per_hour: 10
    requires_approval_if: "amount > 100"
  file_write:
    allow: false
"#;
        let out = PolicyValidator::from_yaml(yaml).unwrap();
        let doc = &out.document;

        assert_eq!(doc.version, Some("1.0".to_string()));

        let np = doc.network.as_ref().unwrap();
        assert_eq!(np.allowlist.len(), 2);

        let sp = doc.schedule.as_ref().unwrap();
        let ah = sp.active_hours.as_ref().unwrap();
        assert_eq!(ah.timezone, "Asia/Taipei");

        let bp = doc.budget.as_ref().unwrap();
        assert_eq!(bp.daily_limit_usd, Some(25.0));

        let dp = doc.data.as_ref().unwrap();
        assert_eq!(dp.sensitive_patterns.len(), 1);

        assert!(doc.tools["bash"].allow);
        assert_eq!(doc.tools["bash"].limit_per_hour, Some(10));
        assert!(!doc.tools["file_write"].allow);

        assert!(out.warnings.is_empty());
    }

    #[test]
    fn full_policy_with_multiple_errors_collects_all() {
        let yaml = r#"
network:
  allowlist:
    - ""
budget:
  daily_limit_usd: 0.0
data:
  sensitive_patterns:
    - "[bad"
"#;
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.field == "network.allowlist[0]"));
        assert!(errs.iter().any(|e| e.field == "budget.daily_limit_usd"));
        assert!(errs.iter().any(|e| e.field == "data.sensitive_patterns[0]"));
    }

    // ── Malformed YAML ──────────────────────────────────────────────────────

    #[test]
    fn malformed_yaml_returns_parse_error() {
        let yaml = ":\n  bad: [unclosed\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert_eq!(errs[0].field, "(document)");
        assert!(errs[0].message.contains("YAML parse error"));
    }

    #[test]
    fn empty_document_is_valid_with_no_errors() {
        let yaml = "{}\n";
        let result = PolicyValidator::from_yaml(yaml);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert!(out.warnings.is_empty());
        assert!(out.document.network.is_none());
    }
}
