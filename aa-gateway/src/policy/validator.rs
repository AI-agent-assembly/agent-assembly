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
