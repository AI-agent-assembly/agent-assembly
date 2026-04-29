//! Integration tests for the policy endpoints.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const VALID_POLICY_YAML: &str = r#"
apiVersion: agent-assembly.dev/v1alpha1
kind: GovernancePolicy
metadata:
  name: test-policy
  version: "1.0.0"
spec:
  rules: []
"#;

const INVALID_POLICY_YAML: &str = "this is not valid yaml: [";

#[tokio::test]
async fn create_policy_returns_201_for_valid_yaml() {
    let app = common::test_app();

    let body = serde_json::json!({ "policy_yaml": VALID_POLICY_YAML });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/policies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["active"], true);
    assert!(json["version"].as_str().is_some());
}

#[tokio::test]
async fn create_policy_returns_400_for_invalid_yaml() {
    let app = common::test_app();

    let body = serde_json::json!({ "policy_yaml": INVALID_POLICY_YAML });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/policies")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_policies_returns_200() {
    let app = common::test_app();

    let response = app
        .oneshot(Request::builder().uri("/api/v1/policies").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].as_array().is_some());
}
