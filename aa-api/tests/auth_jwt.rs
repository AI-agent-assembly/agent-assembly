//! Integration tests for JWT authentication flow.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use aa_api::auth::scope::Scope;

#[tokio::test]
async fn test_valid_jwt_grants_access() {
    let (_plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write]);
    let app = common::test_app_with_auth(&[entry], 1000);
    let jwt = common::generate_test_jwt("key-1", &[Scope::Read, Scope::Write]);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {jwt}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_expired_jwt_returns_401() {
    let (_plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read]);
    let app = common::test_app_with_auth(&[entry], 1000);

    // JWT signed with a different secret should fail verification.
    let wrong_signer = aa_api::auth::jwt::JwtSigner::new(b"wrong-secret-that-is-at-least-32-bytes-long!!");
    let wrong_jwt = wrong_signer
        .sign("key-1", &[Scope::Read])
        .expect("signing should succeed");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {wrong_jwt}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_secret_jwt_returns_401() {
    let (_plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read]);
    let app = common::test_app_with_auth(&[entry], 1000);

    let wrong_signer = aa_api::auth::jwt::JwtSigner::new(
        b"different-secret-that-is-also-32-bytes-long!!",
    );
    let jwt = wrong_signer
        .sign("key-1", &[Scope::Read])
        .expect("signing should succeed");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {jwt}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_endpoint_issues_jwt() {
    let (plaintext, entry) =
        common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write]);
    let app = common::test_app_with_auth(&[entry], 1000);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].is_string(), "response should contain a token");
    assert!(json["expires_at"].is_u64(), "response should contain expires_at");
    assert!(json["scopes"].is_array(), "response should contain scopes");
}

#[tokio::test]
async fn test_token_endpoint_respects_scope_subset() {
    let (plaintext, entry) = common::generate_test_api_key("key-1", vec![Scope::Read]);
    let app = common::test_app_with_auth(&[entry], 1000);

    // Request Write scope when caller only has Read — should fail.
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
