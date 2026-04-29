//! Tower middleware stack for the API server.
//!
//! Middleware is applied in this order (outermost first):
//! 1. Request ID injection (`x-request-id`)
//! 2. Structured tracing (logs method, path, status, duration, request_id)
//! 3. CORS (allow dashboard origin)
//! 4. Response compression (gzip)
//!
//! Authentication is handled by FromRequestParts extractors (see auth module),
//! not middleware layers.

pub mod compression;
pub mod cors;
pub mod request_id;
pub mod tracing;

use axum::Router;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};

use self::request_id::UuidRequestId;

/// Apply the full middleware stack to the given router.
pub fn apply_middleware(router: Router) -> Router {
    router
        .layer(self::compression::compression_layer())
        .layer(self::cors::cors_layer())
        .layer(self::tracing::trace_layer())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(UuidRequestId))
}
