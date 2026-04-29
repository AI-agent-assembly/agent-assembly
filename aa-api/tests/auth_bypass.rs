//! Integration tests for AA_AUTH=off bypass mode.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_bypass_mode_allows_unauthenticated() {
    let app = common::test_app_no_auth();

    // No credentials at all — should still succeed in bypass mode.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_bypass_mode_grants_admin_scope() {
    let app = common::test_app_no_auth();

    // Request a token with admin scope — bypass caller has all scopes.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scopes":["admin"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let scopes = json["scopes"].as_array().expect("scopes should be array");
    assert!(
        scopes.iter().any(|s| s.as_str() == Some("admin")),
        "bypass caller should have admin scope"
    );
}
