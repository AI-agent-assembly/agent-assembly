//! Structured request/response tracing middleware.
//!
//! Logs method, path, status, duration_ms, and request_id for every
//! request using `tower-http::trace`.

use axum::http::Request;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, Span};

/// Build a [`TraceLayer`] that logs request and response metadata.
pub fn trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    impl Fn(&Request<axum::body::Body>) -> Span + Clone,
    (),
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<axum::body::Body>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-");
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                path = %request.uri().path(),
                request_id = %request_id,
            )
        })
        .on_request(())
        .on_response(DefaultOnResponse::new().level(Level::INFO).include_headers(false))
}
