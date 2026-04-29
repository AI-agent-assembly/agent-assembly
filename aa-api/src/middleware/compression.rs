//! Response compression middleware.
//!
//! Applies gzip compression to HTTP response bodies.

use tower_http::compression::CompressionLayer;

/// Build a [`CompressionLayer`] with gzip encoding.
pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new().gzip(true)
}
