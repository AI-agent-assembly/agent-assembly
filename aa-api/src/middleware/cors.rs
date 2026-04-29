//! CORS middleware configuration.
//!
//! Allows the dashboard origin (`http://localhost:3000` in dev) to make
//! cross-origin requests to the API.

use axum::http::{header, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Build a [`CorsLayer`] configured for dashboard access.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "http://localhost:3000".parse().expect("valid origin"),
        ]))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ])
        .allow_credentials(true)
}
