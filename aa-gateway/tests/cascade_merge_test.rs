//! Integration tests for merge_decisions most-restrictive-wins semantics (AAASM-961).

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use aa_core::identity::{AgentId, SessionId};
use aa_core::{AgentContext, GovernanceAction, GovernanceLevel};
use aa_gateway::engine::decision::{merge_decisions, PolicyDecision};
use aa_gateway::policy::document::PolicyDocument;
use aa_gateway::policy::scope::PolicyScope;

fn allow_doc(scope: PolicyScope) -> Arc<PolicyDocument> {
    Arc::new(PolicyDocument {
        name: None,
        policy_version: None,
        version: None,
        scope,
        network: None,
        schedule: None,
        budget: None,
        data: None,
        approval_timeout_secs: 300,
        tools: HashMap::new(),
    })
}

fn deny_tool_doc(scope: PolicyScope, tool_name: &str) -> Arc<PolicyDocument> {
    use aa_gateway::policy::document::ToolPolicy;
    let mut tools = HashMap::new();
    tools.insert(
        tool_name.to_string(),
        ToolPolicy {
            allow: false,
            requires_approval_if: None,
            limit_per_hour: None,
        },
    );
    Arc::new(PolicyDocument {
        name: None,
        policy_version: None,
        version: None,
        scope,
        network: None,
        schedule: None,
        budget: None,
        data: None,
        approval_timeout_secs: 300,
        tools,
    })
}

fn approval_tool_doc(scope: PolicyScope, tool_name: &str, timeout: u32) -> Arc<PolicyDocument> {
    use aa_gateway::policy::document::ToolPolicy;
    let mut tools = HashMap::new();
    tools.insert(
        tool_name.to_string(),
        ToolPolicy {
            allow: true,
            requires_approval_if: Some("true".to_string()),
            limit_per_hour: None,
        },
    );
    Arc::new(PolicyDocument {
        name: None,
        policy_version: None,
        version: None,
        scope,
        network: None,
        schedule: None,
        budget: None,
        data: None,
        approval_timeout_secs: timeout,
        tools,
    })
}

fn make_ctx() -> AgentContext {
    AgentContext {
        agent_id: AgentId::from_bytes([1u8; 16]),
        session_id: SessionId::from_bytes([0u8; 16]),
        pid: 0,
        started_at: aa_core::time::Timestamp::from_nanos(0),
        metadata: BTreeMap::new(),
        governance_level: GovernanceLevel::default(),
    }
}

fn tool_action(name: &str) -> GovernanceAction {
    GovernanceAction::ToolCall {
        name: name.to_string(),
        args: String::new(),
    }
}

// 1. Empty cascade returns fail-closed Deny.
#[test]
fn empty_cascade_is_deny_fail_closed() {
    let ctx = make_ctx();
    let action = tool_action("bash");
    let result = merge_decisions(&[], &ctx, &action);
    assert!(
        matches!(result, PolicyDecision::Deny { .. }),
        "empty cascade must be fail-closed Deny"
    );
}

// 2. Single Allow doc returns Allow.
#[test]
fn single_allow_doc_returns_allow() {
    let ctx = make_ctx();
    let action = tool_action("bash");
    let cascade = vec![allow_doc(PolicyScope::Global)];
    let result = merge_decisions(&cascade, &ctx, &action);
    assert_eq!(result, PolicyDecision::Allow);
}

// 3. Deny in any scope short-circuits and wins over Allow docs.
#[test]
fn deny_in_any_scope_wins_over_allow() {
    let ctx = make_ctx();
    let action = tool_action("bash");
    let cascade = vec![
        allow_doc(PolicyScope::Global),
        deny_tool_doc(PolicyScope::Org("acme".into()), "bash"),
        allow_doc(PolicyScope::Agent(AgentId::from_bytes([1u8; 16]))),
    ];
    let result = merge_decisions(&cascade, &ctx, &action);
    assert!(
        matches!(result, PolicyDecision::Deny { .. }),
        "Deny must win over Allow"
    );
}

// 4. RequireApproval upgrades Allow; first (broadest-scope) RequireApproval is kept.
#[test]
fn require_approval_upgrades_allow_first_approval_kept() {
    let ctx = make_ctx();
    let action = tool_action("deploy");
    let cascade = vec![
        allow_doc(PolicyScope::Global),
        approval_tool_doc(PolicyScope::Org("acme".into()), "deploy", 600),
        approval_tool_doc(PolicyScope::Team("platform".into()), "deploy", 120),
    ];
    let result = merge_decisions(&cascade, &ctx, &action);
    match result {
        PolicyDecision::RequireApproval { timeout_secs, .. } => {
            assert_eq!(
                timeout_secs, 600,
                "first RequireApproval (broadest scope, 600s) must be kept"
            );
        }
        other => panic!("expected RequireApproval, got {other:?}"),
    }
}

// 5. Deny overrides RequireApproval — most restrictive wins.
#[test]
fn deny_overrides_require_approval() {
    let ctx = make_ctx();
    let action = tool_action("deploy");
    let cascade = vec![
        allow_doc(PolicyScope::Global),
        approval_tool_doc(PolicyScope::Org("acme".into()), "deploy", 300),
        deny_tool_doc(PolicyScope::Team("platform".into()), "deploy"),
    ];
    let result = merge_decisions(&cascade, &ctx, &action);
    assert!(
        matches!(result, PolicyDecision::Deny { .. }),
        "Deny must beat RequireApproval"
    );
}
