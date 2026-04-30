//! `aasm logs` — paginated audit log viewer and real-time log tail.

use std::process::ExitCode;

use clap::Args;

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod fetch;
pub mod follow;
pub mod format;
pub mod types;

use types::LogEventType;

/// Arguments for the `aasm logs` subcommand.
#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Stream events in real-time (like `tail -f`). Connects via WebSocket.
    #[arg(long, short = 'f')]
    pub follow: bool,

    /// Filter by agent identifier.
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by event type (comma-separated). Accepted: violation, approval, budget.
    #[arg(long, value_delimiter = ',')]
    pub r#type: Option<Vec<LogEventType>>,

    /// Show events after this duration or ISO 8601 timestamp (e.g. `30m`, `2h`, `2026-04-30T10:00:00Z`).
    #[arg(long)]
    pub since: Option<String>,

    /// Show events before this ISO 8601 timestamp.
    #[arg(long)]
    pub until: Option<String>,

    /// Maximum number of entries to return in non-follow mode.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    /// Disable colour output.
    #[arg(long)]
    pub no_color: bool,

    /// Override the global output format for this command.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}

/// Dispatch the `aasm logs` command to fetch or follow mode.
pub fn dispatch(args: LogsArgs, ctx: &ResolvedContext) -> ExitCode {
    if args.follow {
        follow::run(args, ctx)
    } else {
        fetch::run(args, ctx)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    /// Minimal top-level parser for testing `logs` subcommand argument parsing.
    #[derive(Parser)]
    #[command(name = "aasm")]
    struct TestCli {
        #[command(subcommand)]
        command: TestCommands,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        Logs(super::LogsArgs),
    }

    fn parse(args: &[&str]) -> super::LogsArgs {
        let cli = TestCli::parse_from(args);
        match cli.command {
            TestCommands::Logs(a) => a,
        }
    }

    #[test]
    fn parse_defaults() {
        let args = parse(&["aasm", "logs"]);
        assert!(!args.follow);
        assert!(args.agent.is_none());
        assert!(args.r#type.is_none());
        assert!(args.since.is_none());
        assert!(args.until.is_none());
        assert_eq!(args.limit, 50);
        assert!(!args.no_color);
        assert!(args.output.is_none());
    }

    #[test]
    fn parse_follow_short_flag() {
        let args = parse(&["aasm", "logs", "-f"]);
        assert!(args.follow);
    }

    #[test]
    fn parse_follow_long_flag() {
        let args = parse(&["aasm", "logs", "--follow"]);
        assert!(args.follow);
    }

    #[test]
    fn parse_agent_filter() {
        let args = parse(&["aasm", "logs", "--agent", "aa001"]);
        assert_eq!(args.agent.as_deref(), Some("aa001"));
    }

    #[test]
    fn parse_single_type_filter() {
        let args = parse(&["aasm", "logs", "--type", "violation"]);
        let types = args.r#type.unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].as_api_str(), "violation");
    }

    #[test]
    fn parse_comma_separated_type_filter() {
        let args = parse(&["aasm", "logs", "--type", "violation,budget"]);
        let types = args.r#type.unwrap();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0].as_api_str(), "violation");
        assert_eq!(types[1].as_api_str(), "budget");
    }

    #[test]
    fn parse_invalid_type_rejected() {
        let result = TestCli::try_parse_from(["aasm", "logs", "--type", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_limit() {
        let args = parse(&["aasm", "logs", "--limit", "100"]);
        assert_eq!(args.limit, 100);
    }

    #[test]
    fn parse_no_color() {
        let args = parse(&["aasm", "logs", "--no-color"]);
        assert!(args.no_color);
    }

    #[test]
    fn parse_output_json() {
        let args = parse(&["aasm", "logs", "--output", "json"]);
        assert!(matches!(args.output, Some(crate::output::OutputFormat::Json)));
    }

    #[test]
    fn parse_combined_flags() {
        let args = parse(&[
            "aasm", "logs", "-f", "--agent", "aa001", "--type", "violation,approval",
            "--no-color", "--output", "json",
        ]);
        assert!(args.follow);
        assert_eq!(args.agent.as_deref(), Some("aa001"));
        assert_eq!(args.r#type.as_ref().unwrap().len(), 2);
        assert!(args.no_color);
        assert!(matches!(args.output, Some(crate::output::OutputFormat::Json)));
    }
}
