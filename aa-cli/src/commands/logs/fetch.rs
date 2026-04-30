//! Non-follow mode: paginated audit log query via REST.

use std::process::ExitCode;

use serde::Deserialize;

use crate::config::ResolvedContext;

use super::LogsArgs;

/// Paginated response envelope from `GET /api/v1/logs`.
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<LogEntry>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

/// A single audit log entry as returned by the REST API.
#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp: String,
    pub agent_id: String,
    pub session_id: String,
    pub event_type: String,
    pub payload: String,
}

/// Fetch paginated log entries from `GET /api/v1/logs`.
pub fn run(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
}
