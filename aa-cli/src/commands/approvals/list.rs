//! `aasm approvals list` — list pending approval requests.

use clap::Args;
use comfy_table::{Cell, Color, Table};

use crate::output::OutputFormat;

use super::models::{compute_timeout_color, format_countdown, ApprovalResponse, TimeoutColor};

/// Arguments for the `aasm approvals list` subcommand.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Output format override for this subcommand.
    #[arg(long, value_enum)]
    pub output: Option<OutputFormat>,
}

/// Render a list of approval responses as a colored table to stdout.
///
/// Columns: ID, AGENT, ACTION, CONDITION, SUBMITTED_AT, TIMEOUT_IN.
/// The TIMEOUT_IN column is color-coded: red < 60s, yellow 60-180s, green > 180s.
pub fn render_approvals_table(items: &[ApprovalResponse], now_epoch: i64) {
    let mut table = Table::new();
    table.set_header(vec!["ID", "AGENT", "ACTION", "CONDITION", "SUBMITTED_AT", "TIMEOUT_IN"]);

    for item in items {
        let submitted_epoch = chrono::DateTime::parse_from_rfc3339(&item.created_at)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        // The API does not expose timeout_secs directly; estimate as 300s default.
        let timeout_secs: i64 = 300;
        let remaining = (submitted_epoch + timeout_secs) - now_epoch;
        let color = match compute_timeout_color(remaining) {
            TimeoutColor::Red => Color::Red,
            TimeoutColor::Yellow => Color::Yellow,
            TimeoutColor::Green => Color::Green,
        };

        table.add_row(vec![
            Cell::new(&item.id),
            Cell::new(&item.agent_id),
            Cell::new(&item.action),
            Cell::new(&item.reason),
            Cell::new(&item.created_at),
            Cell::new(format_countdown(remaining)).fg(color),
        ]);
    }

    println!("{table}");
}
