//! Integration tests for scope enforcement.
//!
//! These tests verify that the token endpoint correctly enforces scope
//! constraints when issuing JWTs with specific scope subsets.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use aa_api::auth::scope::Scope;

#[tokio::test]
async fn test_read_scope_allows_token_with_read() {
    let (plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read]);
    let app = common::test_app_with_auth(&[entry], 1000);

    // Read-only caller requests a token with read scope — allowed.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scopes":["read"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_read_scope_blocks_write_token() {
    let (plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read]);
    let app = common::test_app_with_auth(&[entry], 1000);

    // Read-only caller requests a write-scoped token — forbidden.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scopes":["write"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_write_scope_allows_write_token() {
    let (plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write]);
    let app = common::test_app_with_auth(&[entry], 1000);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scopes":["write"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_scope_allows_all_token_scopes() {
    let (plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write, Scope::Admin]);
    let app = common::test_app_with_auth(&[entry], 1000);

    // Admin caller requests all scopes — allowed.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scopes":["read","write","admin"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let scopes = json["scopes"].as_array().expect("scopes should be array");
    assert_eq!(scopes.len(), 3);
}
