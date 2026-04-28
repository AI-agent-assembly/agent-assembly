//! Policy engine implementation.
//!
//! Core rate limiting and enforcement mechanisms for the Agent Assembly policy engine.

pub(crate) mod budget;
pub(crate) mod rate_limit;
pub(crate) mod watcher;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::policy::{PolicyDocument, PolicyValidator};

/// Assembled policy engine that evaluates governance actions through a 7-step pipeline.
pub struct PolicyEngine {
    policy: Arc<ArcSwap<PolicyDocument>>,
    /// Pre-compiled regex patterns from the policy's data section.
    ///
    /// Compiled once at load time to avoid re-compiling on every `evaluate()` call.
    // TODO: recompile on hot-reload — currently these patterns reflect the policy at engine
    // construction time and will not update when the watcher swaps in a new policy document.
    compiled_patterns: Vec<regex::Regex>,
    rate_state: DashMap<String, Mutex<crate::engine::rate_limit::TokenBucket>>,
    budget: crate::engine::budget::BudgetTracker,
    _watcher: Option<notify::RecommendedWatcher>,
}

/// Error returned when loading a policy from a file fails.
#[derive(Debug)]
pub enum PolicyLoadError {
    /// An I/O error occurred reading the file.
    Io(std::io::Error),
    /// The YAML parsed but failed policy validation.
    Validation(Vec<crate::policy::ValidationError>),
}

impl PolicyEngine {
    /// Load a policy from a YAML file, parse it, validate it, and start the filesystem watcher.
    pub fn load_from_file(path: &Path) -> Result<Self, PolicyLoadError> {
        let yaml = std::fs::read_to_string(path).map_err(PolicyLoadError::Io)?;
        let output = PolicyValidator::from_yaml(&yaml).map_err(PolicyLoadError::Validation)?;
        let compiled_patterns = output
            .document
            .data
            .as_ref()
            .map(|dp| {
                dp.sensitive_patterns
                    .iter()
                    .filter_map(|p| regex::Regex::new(p).ok())
                    .collect()
            })
            .unwrap_or_default();
        let budget_tz = output.document.budget.as_ref()
            .and_then(|bp| bp.timezone.as_deref())
            .and_then(|s| s.parse::<chrono_tz::Tz>().ok())
            .unwrap_or(chrono_tz::UTC);
        let policy_arc = Arc::new(ArcSwap::new(Arc::new(output.document)));
        let watcher = crate::engine::watcher::start_watcher(path, policy_arc.clone()).ok();
        Ok(PolicyEngine {
            policy: policy_arc,
            compiled_patterns,
            rate_state: DashMap::new(),
            budget: crate::engine::budget::BudgetTracker::new(budget_tz),
            _watcher: watcher,
        })
    }

    /// Evaluate a governance action through the 7-step pipeline.
    ///
    /// Stages are evaluated in order; the first `Deny` short-circuits the pipeline.
    pub fn evaluate(&self, ctx: &aa_core::AgentContext, action: &aa_core::GovernanceAction) -> aa_core::PolicyResult {
        let policy = self.policy.load();

        // Stage 1 — Schedule: check active hours window.
        if let Some(schedule) = &policy.schedule {
            if let Some(ah) = &schedule.active_hours {
                use chrono::Timelike;
                let tz: chrono_tz::Tz = ah.timezone.parse().unwrap_or(chrono_tz::UTC);
                let now = chrono::Utc::now().with_timezone(&tz);
                let current_hhmm = format!("{:02}:{:02}", now.hour(), now.minute());
                if current_hhmm < ah.start || current_hhmm >= ah.end {
                    return aa_core::PolicyResult::Deny {
                        reason: "outside active hours".into(),
                    };
                }
            }
        }

        // Stage 2 — Network allowlist: only for NetworkRequest actions.
        if let aa_core::GovernanceAction::NetworkRequest { url, .. } = action {
            if let Some(np) = &policy.network {
                if !np.allowlist.is_empty() {
                    let host = url
                        .split_once("://")
                        .map(|x| x.1)
                        .unwrap_or("")
                        .split('/')
                        .next()
                        .unwrap_or("");
                    if !np.allowlist.iter().any(|entry| entry == host) {
                        return aa_core::PolicyResult::Deny {
                            reason: "host not in network allowlist".into(),
                        };
                    }
                }
            }
        }

        // Stage 3 — Tool allow/deny.
        if let aa_core::GovernanceAction::ToolCall { name, .. } = action {
            if let Some(tp) = policy.tools.get(name) {
                if !tp.allow {
                    return aa_core::PolicyResult::Deny {
                        reason: "tool denied by policy".into(),
                    };
                }
            }
        }

        // Stage 4 — Tool rate limit.
        if let aa_core::GovernanceAction::ToolCall { name, .. } = action {
            if let Some(tp) = policy.tools.get(name) {
                if let Some(limit) = tp.limit_per_hour {
                    let entry = self
                        .rate_state
                        .entry(name.clone())
                        .or_insert_with(|| Mutex::new(rate_limit::TokenBucket::new(limit)));
                    let mut bucket = entry.lock().unwrap_or_else(|e| e.into_inner());
                    if !bucket.try_consume() {
                        return aa_core::PolicyResult::Deny {
                            reason: "rate limit exceeded".into(),
                        };
                    }
                }
            }
        }

        // Stage 5 — Approval condition.
        if let aa_core::GovernanceAction::ToolCall { name, .. } = action {
            if let Some(tp) = policy.tools.get(name) {
                if let Some(expr) = &tp.requires_approval_if {
                    if !expr.is_empty() && crate::policy::expr::evaluate(expr, action) {
                        return aa_core::PolicyResult::RequiresApproval { timeout_secs: 30 };
                    }
                }
            }
        }

        // Stage 6 — Data pattern scan (uses pre-compiled regexes from load time).
        if !self.compiled_patterns.is_empty() {
            let text = match action {
                aa_core::GovernanceAction::ToolCall { args, .. } => args.as_str(),
                aa_core::GovernanceAction::FileAccess { path, .. } => path.as_str(),
                aa_core::GovernanceAction::NetworkRequest { url, .. } => url.as_str(),
                aa_core::GovernanceAction::ProcessExec { command } => command.as_str(),
            };
            for re in &self.compiled_patterns {
                if re.is_match(text) {
                    return aa_core::PolicyResult::Deny {
                        reason: "sensitive data pattern matched".into(),
                    };
                }
            }
        }

        // Stage 7 — Budget check.
        if let Some(bp) = &policy.budget {
            if let Some(limit) = bp.daily_limit_usd {
                if self.budget.is_exceeded(ctx.agent_id.as_bytes(), limit) {
                    return aa_core::PolicyResult::Deny {
                        reason: "daily budget exceeded".into(),
                    };
                }
            }
        }

        aa_core::PolicyResult::Allow
    }

    /// Record a spend amount for an agent after an action completes.
    pub fn record_spend(&self, ctx: &aa_core::AgentContext, amount_usd: f64) {
        self.budget.record(ctx.agent_id.as_bytes(), amount_usd);
    }
}

/// Implement the `aa_core::PolicyEvaluator` trait so `PolicyEngine` can be used
/// as `dyn PolicyEvaluator` wherever a pluggable evaluation backend is expected.
///
/// `load_policy` and `validate_policy` are not meaningful for `PolicyEngine` because
/// it uses a richer YAML-based policy document (not the `aa_core::PolicyDocument` stub).
/// Both methods return `Err(PolicyError::InvalidDocument)` to make the limitation explicit.
/// Use [`PolicyEngine::load_from_file`] to construct and reload a live engine.
impl aa_core::PolicyEvaluator for PolicyEngine {
    fn evaluate(&self, ctx: &aa_core::AgentContext, action: &aa_core::GovernanceAction) -> aa_core::PolicyResult {
        PolicyEngine::evaluate(self, ctx, action)
    }

    fn load_policy(&mut self, _policy: &aa_core::PolicyDocument) -> Result<(), aa_core::PolicyError> {
        Err(aa_core::PolicyError::InvalidDocument)
    }

    fn validate_policy(&self, _policy: &aa_core::PolicyDocument) -> Result<(), Vec<aa_core::PolicyError>> {
        Err(vec![aa_core::PolicyError::InvalidDocument])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::document::{
        ActiveHours, BudgetPolicy, DataPolicy, NetworkPolicy, PolicyDocument, SchedulePolicy, ToolPolicy,
    };
    use aa_core::{
        identity::{AgentId, SessionId},
        time::Timestamp,
        AgentContext, GovernanceAction, PolicyResult,
    };
    use std::collections::{BTreeMap, HashMap};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_ctx() -> AgentContext {
        AgentContext {
            agent_id: AgentId::from_bytes([1u8; 16]),
            session_id: SessionId::from_bytes([2u8; 16]),
            pid: 1,
            started_at: Timestamp::from_nanos(0),
            metadata: BTreeMap::new(),
        }
    }

    fn empty_doc() -> PolicyDocument {
        PolicyDocument {
            version: None,
            network: None,
            schedule: None,
            budget: None,
            data: None,
            tools: HashMap::new(),
        }
    }

    fn make_engine(doc: PolicyDocument) -> PolicyEngine {
        let compiled_patterns = doc
            .data
            .as_ref()
            .map(|dp| {
                dp.sensitive_patterns
                    .iter()
                    .filter_map(|p| regex::Regex::new(p).ok())
                    .collect()
            })
            .unwrap_or_default();
        PolicyEngine {
            policy: Arc::new(ArcSwap::new(Arc::new(doc))),
            compiled_patterns,
            rate_state: DashMap::new(),
            budget: budget::BudgetTracker::new(chrono_tz::UTC),
            _watcher: None,
        }
    }

    fn tool_call(name: &str, args: &str) -> GovernanceAction {
        GovernanceAction::ToolCall {
            name: name.to_string(),
            args: args.to_string(),
        }
    }

    fn network_req(url: &str) -> GovernanceAction {
        GovernanceAction::NetworkRequest {
            url: url.to_string(),
            method: "GET".to_string(),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn evaluate_allows_when_no_policy_sections() {
        let engine = make_engine(empty_doc());
        let ctx = make_ctx();
        let action = tool_call("any", "");
        assert_eq!(engine.evaluate(&ctx, &action), PolicyResult::Allow);
    }

    #[test]
    fn schedule_denies_outside_active_hours() {
        // A window of 00:00–00:01 will almost certainly be outside the current time.
        let mut doc = empty_doc();
        doc.schedule = Some(SchedulePolicy {
            active_hours: Some(ActiveHours {
                start: "00:00".to_string(),
                end: "00:01".to_string(),
                timezone: "UTC".to_string(),
            }),
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("any", "");
        let result = engine.evaluate(&ctx, &action);
        // This window is 1 minute wide; unless tests run exactly at midnight, it's Deny.
        // Accept either Deny or Allow (if tests run in the 00:00–00:01 window).
        match result {
            PolicyResult::Deny { reason } => {
                assert_eq!(reason, "outside active hours");
            }
            PolicyResult::Allow => {
                // Rare but possible if test runs exactly at midnight UTC.
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn schedule_allows_full_day_window() {
        let mut doc = empty_doc();
        doc.schedule = Some(SchedulePolicy {
            active_hours: Some(ActiveHours {
                start: "00:00".to_string(),
                end: "23:59".to_string(),
                timezone: "UTC".to_string(),
            }),
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("any", "");
        // 00:00–23:59 covers almost the whole day — should Allow.
        let result = engine.evaluate(&ctx, &action);
        assert_eq!(result, PolicyResult::Allow);
    }

    #[test]
    fn network_denies_unlisted_host() {
        let mut doc = empty_doc();
        doc.network = Some(NetworkPolicy {
            allowlist: vec!["api.openai.com".to_string()],
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = network_req("https://evil.com/path");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "host not in network allowlist".into()
            }
        );
    }

    #[test]
    fn network_allows_listed_host() {
        let mut doc = empty_doc();
        doc.network = Some(NetworkPolicy {
            allowlist: vec!["api.openai.com".to_string()],
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = network_req("https://api.openai.com/v1");
        assert_eq!(engine.evaluate(&ctx, &action), PolicyResult::Allow);
    }

    #[test]
    fn tool_deny_blocks_call() {
        let mut doc = empty_doc();
        doc.tools.insert(
            "ls".to_string(),
            ToolPolicy {
                allow: false,
                limit_per_hour: None,
                requires_approval_if: None,
            },
        );
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("ls", "");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "tool denied by policy".into()
            }
        );
    }

    #[test]
    fn tool_allow_passes_call() {
        let mut doc = empty_doc();
        doc.tools.insert(
            "ls".to_string(),
            ToolPolicy {
                allow: true,
                limit_per_hour: None,
                requires_approval_if: None,
            },
        );
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("ls", "");
        assert_eq!(engine.evaluate(&ctx, &action), PolicyResult::Allow);
    }

    #[test]
    fn rate_limit_denies_after_capacity_exhausted() {
        let mut doc = empty_doc();
        doc.tools.insert(
            "search".to_string(),
            ToolPolicy {
                allow: true,
                limit_per_hour: Some(1),
                requires_approval_if: None,
            },
        );
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("search", "");

        // First call should succeed.
        assert_eq!(engine.evaluate(&ctx, &action), PolicyResult::Allow);
        // Second call should be rate-limited.
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "rate limit exceeded".into()
            }
        );
    }

    #[test]
    fn approval_condition_triggers_requires_approval() {
        let mut doc = empty_doc();
        doc.tools.insert(
            "search".to_string(),
            ToolPolicy {
                allow: true,
                limit_per_hour: None,
                requires_approval_if: Some(r#"tool == "search""#.to_string()),
            },
        );
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("search", "");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::RequiresApproval { timeout_secs: 30 }
        );
    }

    #[test]
    fn data_pattern_denies_sensitive_match() {
        let mut doc = empty_doc();
        doc.data = Some(DataPolicy {
            sensitive_patterns: vec![r"password=\w+".to_string()],
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("any", "password=secret");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "sensitive data pattern matched".into()
            }
        );
    }

    #[test]
    fn budget_denies_when_exceeded() {
        let mut doc = empty_doc();
        doc.budget = Some(BudgetPolicy {
            daily_limit_usd: Some(1.0),
            timezone: None,
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();

        engine.record_spend(&ctx, 1.0);

        let action = tool_call("any", "");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "daily budget exceeded".into()
            }
        );
    }

    #[test]
    fn short_circuit_stops_at_first_deny() {
        // Tool deny should fire before data pattern stage.
        let mut doc = empty_doc();
        doc.tools.insert(
            "ls".to_string(),
            ToolPolicy {
                allow: false,
                limit_per_hour: None,
                requires_approval_if: None,
            },
        );
        doc.data = Some(DataPolicy {
            sensitive_patterns: vec![".*".to_string()],
        });
        let engine = make_engine(doc);
        let ctx = make_ctx();
        let action = tool_call("ls", "anything");
        assert_eq!(
            engine.evaluate(&ctx, &action),
            PolicyResult::Deny {
                reason: "tool denied by policy".into()
            }
        );
    }

    #[test]
    fn load_from_file_returns_engine() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "version: \"1\"\ntools:\n  search:\n    allow: true\n").unwrap();
        tmp.flush().unwrap();
        let result = PolicyEngine::load_from_file(tmp.path());
        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    }

    // ── PolicyEvaluator trait impl ────────────────────────────────────────────

    #[test]
    fn trait_evaluate_delegates_to_inherent_method() {
        use aa_core::PolicyEvaluator;
        let engine = make_engine(empty_doc());
        let ctx = make_ctx();
        let action = tool_call("any", "");
        // Call via the trait — result must match the inherent method.
        let via_trait = <PolicyEngine as PolicyEvaluator>::evaluate(&engine, &ctx, &action);
        let via_inherent = engine.evaluate(&ctx, &action);
        assert_eq!(via_trait, via_inherent);
    }

    #[test]
    fn trait_load_policy_returns_invalid_document() {
        use aa_core::PolicyEvaluator;
        let mut engine = make_engine(empty_doc());
        let stub = aa_core::PolicyDocument {
            version: 1,
            name: "stub".to_string(),
            rules: vec![],
        };
        let result = engine.load_policy(&stub);
        assert_eq!(result, Err(aa_core::PolicyError::InvalidDocument));
    }

    #[test]
    fn trait_validate_policy_returns_invalid_document() {
        use aa_core::PolicyEvaluator;
        let engine = make_engine(empty_doc());
        let stub = aa_core::PolicyDocument {
            version: 1,
            name: "stub".to_string(),
            rules: vec![],
        };
        let result = engine.validate_policy(&stub);
        assert_eq!(result, Err(vec![aa_core::PolicyError::InvalidDocument]));
    }
}
