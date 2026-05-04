//! Integration tests for the F92 Phase B scope index (AAASM-951).
//!
//! Loads each scoped fixture through the YAML validator, feeds the
//! resulting `PolicyDocument` into a fresh `PolicyEngine` via the new
//! `load_policy` method, and asserts that the engine indexes it under
//! the expected `PolicyScope` bucket.

use std::path::Path;

use aa_core::identity::AgentId;
use aa_gateway::engine::PolicyEngine;
use aa_gateway::policy::{PolicyDocument, PolicyScope, PolicyValidator};

/// Parse a scoped fixture from `tests/fixtures/policies/scoped/<name>.yaml`.
fn load_fixture(name: &str) -> PolicyDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/policies/scoped")
        .join(format!("{name}.yaml"));
    let yaml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    PolicyValidator::from_yaml(&yaml)
        .unwrap_or_else(|errs| panic!("validate {}: {errs:?}", path.display()))
        .document
}

/// Build an empty `PolicyEngine` for the index tests. We deliberately
/// avoid `load_from_file` because that constructor is geared toward the
/// single-policy ArcSwap flow; for the index tests we just want a fresh
/// empty `scope_index` and we exercise it via the new `load_policy`
/// inherent method.
fn empty_engine() -> PolicyEngine {
    let yaml = "{}\n";
    let path = std::env::temp_dir().join(format!("aaasm951-empty-{}.yaml", std::process::id()));
    std::fs::write(&path, yaml).unwrap();
    let (alert_tx, _rx) = tokio::sync::broadcast::channel(8);
    PolicyEngine::load_from_file(&path, alert_tx).expect("empty policy loads")
}

#[test]
fn global_scoped_fixture_lands_in_global_bucket() {
    let doc = load_fixture("global");
    assert_eq!(doc.scope, PolicyScope::Global);

    let mut engine = empty_engine();
    let id = engine.load_policy(doc);
    assert_eq!(engine.policies_for_scope(&PolicyScope::Global), &[id]);
}

#[test]
fn org_scoped_fixture_lands_in_matching_org_bucket() {
    let doc = load_fixture("org_acme");
    assert_eq!(doc.scope, PolicyScope::Org("acme".to_owned()));

    let mut engine = empty_engine();
    let id = engine.load_policy(doc);
    assert_eq!(engine.policies_for_scope(&PolicyScope::Org("acme".to_owned())), &[id]);
}

#[test]
fn team_scoped_fixture_lands_in_matching_team_bucket() {
    let doc = load_fixture("team_platform");
    assert_eq!(doc.scope, PolicyScope::Team("platform".to_owned()));

    let mut engine = empty_engine();
    let id = engine.load_policy(doc);
    assert_eq!(
        engine.policies_for_scope(&PolicyScope::Team("platform".to_owned())),
        &[id],
    );
}

#[test]
fn agent_scoped_fixture_lands_in_matching_agent_bucket() {
    const EXPECTED_BYTES: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
    ];
    let doc = load_fixture("agent_specific");
    let expected_scope = PolicyScope::Agent(AgentId::from_bytes(EXPECTED_BYTES));
    assert_eq!(doc.scope, expected_scope);

    let mut engine = empty_engine();
    let id = engine.load_policy(doc);
    assert_eq!(engine.policies_for_scope(&expected_scope), &[id]);
}

#[test]
fn distinct_scoped_fixtures_do_not_contaminate_other_buckets() {
    let global = load_fixture("global");
    let org = load_fixture("org_acme");
    let team = load_fixture("team_platform");

    let mut engine = empty_engine();
    let id_global = engine.load_policy(global);
    let id_org = engine.load_policy(org);
    let id_team = engine.load_policy(team);

    assert_eq!(engine.policies_for_scope(&PolicyScope::Global), &[id_global]);
    assert_eq!(
        engine.policies_for_scope(&PolicyScope::Org("acme".to_owned())),
        &[id_org],
    );
    assert_eq!(
        engine.policies_for_scope(&PolicyScope::Team("platform".to_owned())),
        &[id_team],
    );
}
