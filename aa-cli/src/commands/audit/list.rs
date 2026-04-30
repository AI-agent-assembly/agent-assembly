//! `aasm audit list` — query and display audit log entries.

use std::process::ExitCode;

use clap::Args;

use super::models::AuditResult;
use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for `aasm audit list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Filter by agent identifier.
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by action type (e.g. `tool_call`, `llm_request`).
    #[arg(long)]
    pub action: Option<String>,

    /// Filter by policy decision result.
    #[arg(long, value_enum)]
    pub result: Option<AuditResult>,

    /// Show events after this duration or ISO 8601 timestamp (e.g. `30m`, `2h`, `2026-04-30T10:00:00Z`).
    #[arg(long)]
    pub since: Option<String>,

    /// Show events before this ISO 8601 timestamp.
    #[arg(long)]
    pub until: Option<String>,

    /// Maximum number of entries to return.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

/// Execute `aasm audit list`.
pub fn run(_args: ListArgs, _ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    // Implemented in a subsequent commit.
    ExitCode::SUCCESS
}
