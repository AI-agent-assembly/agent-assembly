//! Follow mode: real-time event streaming via WebSocket.

use std::process::ExitCode;

use crate::config::ResolvedContext;

use super::LogsArgs;

/// Stream events in real-time via WebSocket `/api/v1/ws/events`.
pub fn run(_args: LogsArgs, _ctx: &ResolvedContext) -> ExitCode {
    ExitCode::SUCCESS
}
