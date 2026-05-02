//! Tests that the generated OpenAPI spec is structurally correct.

use utoipa::OpenApi;

#[test]
fn spec_version_is_3_1_0() {
    let spec = aa_api::ApiDoc::openapi();
    assert_eq!(spec.info.version, "0.0.1");
    // utoipa 5.x generates OpenAPI 3.1.0
    let yaml = serde_yaml::to_string(&spec).unwrap();
    assert!(yaml.starts_with("openapi: 3.1.0"));
}

#[test]
fn health_path_exists() {
    let spec = aa_api::ApiDoc::openapi();
    let paths = &spec.paths;
    assert!(
        paths.paths.contains_key("/api/v1/health"),
        "expected /api/v1/health in paths, got: {:?}",
        paths.paths.keys().collect::<Vec<_>>()
    );
}

#[test]
fn health_response_schema_exists() {
    let spec = aa_api::ApiDoc::openapi();
    let schemas = &spec.components.as_ref().expect("components should exist").schemas;
    assert!(schemas.contains_key("HealthResponse"), "HealthResponse schema missing");
    assert!(schemas.contains_key("ProblemDetail"), "ProblemDetail schema missing");
}

#[test]
fn schemas_have_descriptions() {
    let spec = aa_api::ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&spec).unwrap();
    // Doc comments from Rust structs should appear as descriptions
    assert!(
        yaml.contains("Response body for the health endpoint"),
        "HealthResponse description missing from spec"
    );
    assert!(
        yaml.contains("RFC 7807 Problem Details JSON body"),
        "ProblemDetail description missing from spec"
    );
}

#[test]
fn health_get_has_operation_id() {
    let spec = aa_api::ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&spec).unwrap();
    assert!(
        yaml.contains("operationId: health"),
        "health operationId missing from spec"
    );
}

#[test]
fn ws_events_path_exists() {
    let spec = aa_api::ApiDoc::openapi();
    let paths = &spec.paths;
    assert!(
        paths.paths.contains_key("/api/v1/ws/events"),
        "expected /api/v1/ws/events in paths, got: {:?}",
        paths.paths.keys().collect::<Vec<_>>()
    );
}

#[test]
fn ws_events_has_query_params() {
    let spec = aa_api::ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&spec).unwrap();
    // WsQueryParams fields should appear as query parameters
    assert!(
        yaml.contains("operationId: ws_events_handler"),
        "ws operationId missing"
    );
    assert!(yaml.contains("name: types"), "types query param missing");
    assert!(yaml.contains("name: agent_id"), "agent_id query param missing");
    assert!(yaml.contains("name: since"), "since query param missing");
}

#[test]
fn governance_event_schema_exists() {
    let spec = aa_api::ApiDoc::openapi();
    let schemas = &spec.components.as_ref().expect("components should exist").schemas;
    assert!(
        schemas.contains_key("GovernanceEvent"),
        "GovernanceEvent schema missing"
    );
    assert!(schemas.contains_key("EventType"), "EventType schema missing");
    assert!(
        schemas.contains_key("ViolationPayload"),
        "ViolationPayload schema missing"
    );
    assert!(
        schemas.contains_key("ApprovalPayload"),
        "ApprovalPayload schema missing"
    );
    assert!(
        schemas.contains_key("BudgetAlertPayload"),
        "BudgetAlertPayload schema missing"
    );
    assert!(schemas.contains_key("EventPayload"), "EventPayload schema missing");
}

#[test]
fn event_type_enum_variants() {
    let spec = aa_api::ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&spec).unwrap();
    // EventType enum should list all three variants in snake_case
    assert!(yaml.contains("violation"), "violation variant missing from EventType");
    assert!(yaml.contains("approval"), "approval variant missing from EventType");
    assert!(yaml.contains("budget"), "budget variant missing from EventType");
}
