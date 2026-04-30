//! Non-follow mode: paginated audit log query via REST.

use std::process::ExitCode;

use crate::config::ResolvedContext;

use super::LogsArgs;

/// Fetch paginated log entries from `GET /api/v1/logs`.
pub fn run(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
}
