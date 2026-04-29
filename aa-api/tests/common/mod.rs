//! Shared test utilities for aa-api integration tests.

use std::sync::Arc;

use aa_api::events::EventBroadcast;
use aa_api::server::build_app;
use aa_api::state::AppState;
use aa_gateway::budget::pricing::PricingTable;
use aa_gateway::budget::tracker::BudgetTracker;
use aa_gateway::engine::PolicyEngine;
use aa_runtime::approval::ApprovalQueue;
use axum::Router;

/// Build a test `AppState` with minimal real dependencies.
pub fn test_state() -> AppState {
    // PolicyEngine requires a policy file; use a minimal valid policy.
    let policy_dir = std::env::temp_dir().join("aa-api-test-policy");
    std::fs::create_dir_all(&policy_dir).unwrap();
    let policy_path = policy_dir.join("test-policy.yaml");
    std::fs::write(
        &policy_path,
        r#"
apiVersion: agent-assembly.dev/v1alpha1
kind: GovernancePolicy
metadata:
  name: test-policy
  version: "0.1.0"
spec:
  rules: []
"#,
    )
    .unwrap();

    let events = Arc::new(EventBroadcast::default());
    let budget_alert_tx = events.budget_sender();
    let policy_engine =
        Arc::new(PolicyEngine::load_from_file(&policy_path, budget_alert_tx).unwrap());
    let budget_tracker = Arc::new(BudgetTracker::new(
        PricingTable::default_table(),
        None,
        None,
        chrono_tz::UTC,
    ));
    let approval_queue = ApprovalQueue::new();

    AppState {
        policy_engine,
        budget_tracker,
        approval_queue,
        events,
    }
}

/// Build the full app for testing (router + middleware + state).
pub fn test_app() -> Router {
    build_app(test_state())
}
