//! Request ID injection middleware.
//!
//! Assigns a UUID v4 to every incoming request via the `x-request-id` header
//! and propagates it on the response.

use http::HeaderValue;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

/// Generates UUID v4 request identifiers.
#[derive(Clone, Copy)]
pub struct UuidRequestId;

impl MakeRequestId for UuidRequestId {
    fn make_request_id<B>(&mut self, _request: &http::Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        HeaderValue::from_str(&id).ok().map(RequestId::new)
    }
}
