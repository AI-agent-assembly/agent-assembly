//! Integration tests for rate limiting.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use aa_api::auth::scope::Scope;
use aa_api::server::build_app;

#[tokio::test]
async fn test_rate_limit_allows_under_threshold() {
    let (plaintext, entry) =
        common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write]);
    // Set RPM high enough that 3 requests are fine.
    let state = common::test_state_with_auth(
        aa_api::auth::config::AuthMode::On,
        &[entry],
        100,
    );
    let app = build_app(state);

    // Send 3 requests — all should succeed.
    for _ in 0..3 {
        let response = app
            .clone()
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
    }
}

#[tokio::test]
async fn test_rate_limit_returns_429_with_retry_after() {
    let (plaintext, entry) =
        common::generate_test_api_key("key-1", vec![Scope::Read, Scope::Write]);
    // Set RPM to 2 so we can exhaust it quickly.
    let state = common::test_state_with_auth(
        aa_api::auth::config::AuthMode::On,
        &[entry],
        2,
    );
    let app = build_app(state);

    // Exhaust the 2-request limit.
    for _ in 0..2 {
        let response = app
            .clone()
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
    }

    // Third request should be rate-limited.
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

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response.headers().contains_key("retry-after"),
        "429 response should include Retry-After header"
    );
}

#[tokio::test]
async fn test_rate_limit_per_key_isolation() {
    let (plaintext_a, entry_a) =
        common::generate_test_api_key("key-a", vec![Scope::Read, Scope::Write]);
    let (plaintext_b, entry_b) =
        common::generate_test_api_key("key-b", vec![Scope::Read, Scope::Write]);
    // RPM = 2: key-a will exhaust its bucket.
    let state = common::test_state_with_auth(
        aa_api::auth::config::AuthMode::On,
        &[entry_a, entry_b],
        2,
    );
    let app = build_app(state);

    // Exhaust key-a.
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/token")
                    .header("authorization", format!("Bearer {plaintext_a}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // key-a should now be rate-limited.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext_a}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // key-b should still work.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/token")
                .header("authorization", format!("Bearer {plaintext_b}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
