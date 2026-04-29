//! Integration test for RFC 7807 error responses.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn unmatched_route_returns_rfc7807_404() {
    let app = common::test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type, "application/problem+json");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["type"], "about:blank");
    assert_eq!(json["title"], "Not Found");
    assert_eq!(json["status"], 404);
    assert!(json["detail"].as_str().unwrap().contains("nonexistent"));
    assert_eq!(json["instance"], "/api/v1/nonexistent");
}
