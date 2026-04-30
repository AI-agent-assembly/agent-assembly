//! `aasm audit export` — export audit data in CSV or JSON format.

use std::process::ExitCode;

use clap::Args;

use super::models::{AuditResult, ComplianceFormat, ExportFormat};
use crate::config::ResolvedContext;

/// Arguments for `aasm audit export`.
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Export file format.
    #[arg(long, value_enum)]
    pub format: ExportFormat,

    /// Compliance report format (adds metadata headers).
    #[arg(long, value_enum)]
    pub compliance: Option<ComplianceFormat>,

    /// Write output to a file instead of stdout.
    #[arg(long)]
    pub output: Option<String>,

    /// Filter by agent identifier.
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by action type.
    #[arg(long)]
    pub action: Option<String>,

    /// Filter by policy decision result.
    #[arg(long, value_enum)]
    pub result: Option<AuditResult>,

    /// Show events after this duration or ISO 8601 timestamp.
    #[arg(long)]
    pub since: Option<String>,

    /// Show events before this ISO 8601 timestamp.
    #[arg(long)]
    pub until: Option<String>,

    /// Maximum number of entries to fetch.
    #[arg(long, default_value_t = 1000)]
    pub limit: u32,
}

/// Execute `aasm audit export`.
pub fn run(_args: ExportArgs, _ctx: &ResolvedContext) -> ExitCode {
    // Implemented in a subsequent commit.
    ExitCode::SUCCESS
}
