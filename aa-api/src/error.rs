//! RFC 7807 Problem Details error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// RFC 7807 Problem Details JSON body.
#[derive(Debug, Clone, Serialize)]
pub struct ProblemDetail {
    /// URI reference identifying the problem type.
    #[serde(rename = "type")]
    pub type_uri: String,
    /// Short human-readable summary.
    pub title: String,
    /// HTTP status code.
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// URI reference identifying the specific occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ProblemDetail {
    /// Create a `ProblemDetail` from an HTTP status code.
    pub fn from_status(status: StatusCode) -> Self {
        Self {
            type_uri: "about:blank".to_string(),
            title: status
                .canonical_reason()
                .unwrap_or("Unknown Error")
                .to_string(),
            status: status.as_u16(),
            detail: None,
            instance: None,
        }
    }

    /// Attach a human-readable detail message.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Attach the request URI as the instance identifier.
    #[must_use]
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_string(&self).unwrap_or_else(|_| {
            r#"{"type":"about:blank","title":"Internal Server Error","status":500}"#.to_string()
        });

        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/problem+json",
            )],
            body,
        )
            .into_response()
    }
}
