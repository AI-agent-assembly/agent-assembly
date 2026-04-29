//! Authentication and authorization for the API server.
//!
//! Auth is handled via Axum `FromRequestParts` extractors, not middleware
//! layers. The [`AuthenticatedCaller`] extractor validates API keys or JWTs
//! and enforces per-key rate limits. [`RequireScope`] checks scope levels.

pub mod api_key;
pub mod config;
pub mod jwt;
pub mod rate_limit;
pub mod scope;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::ProblemDetail;
use self::scope::Scope;

/// Authentication / authorization errors returned by extractors.
#[derive(Debug)]
pub enum AuthError {
    /// No `Authorization` header was present.
    MissingHeader,
    /// The token could not be validated (bad format, wrong signature, etc.).
    InvalidToken(String),
    /// The token signature was valid but the token has expired.
    ExpiredToken,
    /// The caller has exceeded the per-key rate limit.
    RateLimited {
        /// Seconds until the next request may be accepted.
        retry_after_secs: u64,
    },
    /// The caller's scopes do not satisfy the required scope.
    InsufficientScope {
        /// The scope level that was required.
        required: Scope,
    },
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingHeader => ProblemDetail::from_status(StatusCode::UNAUTHORIZED)
                .with_detail("Missing Authorization header")
                .into_response(),

            AuthError::InvalidToken(reason) => {
                ProblemDetail::from_status(StatusCode::UNAUTHORIZED)
                    .with_detail(format!("Invalid token: {reason}"))
                    .into_response()
            }

            AuthError::ExpiredToken => ProblemDetail::from_status(StatusCode::UNAUTHORIZED)
                .with_detail("Token has expired")
                .into_response(),

            AuthError::RateLimited { retry_after_secs } => {
                let problem = ProblemDetail::from_status(StatusCode::TOO_MANY_REQUESTS)
                    .with_detail(format!(
                        "Rate limit exceeded. Retry after {retry_after_secs} seconds"
                    ));
                let mut response = problem.into_response();
                response.headers_mut().insert(
                    "retry-after",
                    retry_after_secs
                        .to_string()
                        .parse()
                        .expect("integer is valid header value"),
                );
                response
            }

            AuthError::InsufficientScope { required } => {
                ProblemDetail::from_status(StatusCode::FORBIDDEN)
                    .with_detail(format!("Insufficient scope: requires '{required}'"))
                    .into_response()
            }
        }
    }
}
