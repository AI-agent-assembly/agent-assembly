//! Trace models for the session trace query endpoint.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single span within an agent session trace.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TraceSpan {
    /// Span identifier.
    pub span_id: String,
    /// Parent span identifier (links to the calling action).
    pub parent_span_id: Option<String>,
    /// Operation name.
    pub operation: String,
    /// Governance decision result for this span.
    pub decision: Option<String>,
    /// Start time of the span.
    pub start_time: DateTime<Utc>,
    /// End time of the span (if completed).
    pub end_time: Option<DateTime<Utc>>,
}

/// Full trace for one agent session.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TraceResponse {
    /// Session identifier.
    pub session_id: String,
    /// Agent that produced this trace.
    pub agent_id: String,
    /// Ordered list of spans in the session.
    pub spans: Vec<TraceSpan>,
}
