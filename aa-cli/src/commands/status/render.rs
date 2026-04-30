//! Rendering functions for `aasm status` output.

use comfy_table::{ContentArrangement, Table};

use super::models::{AgentRow, ApprovalsSummary, BudgetRow, RuntimeHealth, StatusSnapshot};
use crate::output::OutputFormat;

/// Render the Runtime Health section to stdout.
pub fn render_runtime_health(health: &RuntimeHealth) {
    println!("RUNTIME HEALTH");
    println!("──────────────");
    let indicator = if health.reachable { "✓" } else { "✗" };
    println!("  API:    {indicator} {}", health.status);
    println!();
}

/// Render the Active Agents section as a table to stdout.
pub fn render_agents_table(agents: &[AgentRow]) {
    println!("ACTIVE AGENTS");
    println!("─────────────");
    if agents.is_empty() {
        println!("  (no agents registered)");
        println!();
        return;
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["AGENT_ID", "NAME", "FRAMEWORK", "STATUS", "VIOLATIONS_TODAY"]);
    for agent in agents {
        let status_icon = match agent.status.as_str() {
            "Running" => "●",
            "Idle" => "○",
            "Suspended" => "⚠",
            _ => "?",
        };
        table.add_row(vec![
            &agent.id,
            &agent.name,
            &agent.framework,
            &format!("{status_icon} {}", agent.status),
            &agent.violations_today.to_string(),
        ]);
    }
    println!("{table}");
    println!();
}

/// Render the Pending Approvals section to stdout.
pub fn render_approvals_summary(summary: &ApprovalsSummary) {
    println!("PENDING APPROVALS");
    println!("─────────────────");
    println!("  Count:  {}", summary.pending_count);
    if let Some(ref age) = summary.oldest_pending_age {
        println!("  Oldest: {age} ago");
    }
    println!();
}

/// Render an ASCII bar chart: 20-char wide, `█` for used, `░` for remaining.
///
/// `percentage` is clamped to `0..=100`.
/// Currently used in tests; will be called from `render_budget_table` once
/// per-agent budget data is available from the API.
#[allow(dead_code)]
pub fn format_bar_chart(percentage: u32) -> String {
    let pct = percentage.min(100);
    let filled = (pct as usize * 20) / 100;
    let empty = 20 - filled;
    format!("{}{} {:>3}%", "█".repeat(filled), "░".repeat(empty), pct,)
}

/// Render the Budget Status section to stdout.
pub fn render_budget_table(budget: &BudgetRow) {
    println!("BUDGET STATUS");
    println!("─────────────");
    println!("  Daily spend:   ${}", budget.daily_spend_usd);
    if let Some(ref monthly) = budget.monthly_spend_usd {
        println!("  Monthly spend: ${monthly}");
    }
    println!("  Date:          {}", budget.date);
    println!();
}

/// Render the full status snapshot as JSON to stdout.
pub fn render_status_json(snapshot: &StatusSnapshot) {
    match serde_json::to_string_pretty(snapshot) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("error serializing status to JSON: {e}"),
    }
}

/// Render the full status snapshot using the selected output format.
pub fn render_all(snapshot: &StatusSnapshot, format: OutputFormat) {
    match format {
        OutputFormat::Json => render_status_json(snapshot),
        OutputFormat::Yaml => match serde_yaml::to_string(snapshot) {
            Ok(yaml) => print!("{yaml}"),
            Err(e) => eprintln!("error serializing status to YAML: {e}"),
        },
        OutputFormat::Table => {
            render_runtime_health(&snapshot.runtime);
            render_agents_table(&snapshot.agents);
            render_approvals_summary(&snapshot.approvals);
            render_budget_table(&snapshot.budget);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_chart_at_zero_percent() {
        let bar = format_bar_chart(0);
        assert_eq!(bar, "░░░░░░░░░░░░░░░░░░░░   0%");
    }

    #[test]
    fn bar_chart_at_fifty_percent() {
        let bar = format_bar_chart(50);
        assert_eq!(bar, "██████████░░░░░░░░░░  50%");
    }

    #[test]
    fn bar_chart_at_hundred_percent() {
        let bar = format_bar_chart(100);
        assert_eq!(bar, "████████████████████ 100%");
    }

    #[test]
    fn bar_chart_clamps_above_hundred() {
        let bar = format_bar_chart(150);
        assert_eq!(bar, "████████████████████ 100%");
    }

    #[test]
    fn render_status_json_contains_all_keys() {
        let snapshot = StatusSnapshot {
            runtime: RuntimeHealth {
                reachable: true,
                status: "ok".to_string(),
            },
            agents: vec![],
            approvals: ApprovalsSummary {
                pending_count: 0,
                oldest_pending_age: None,
            },
            budget: BudgetRow {
                daily_spend_usd: "0.00".to_string(),
                monthly_spend_usd: None,
                date: "2026-04-30".to_string(),
            },
        };
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        assert!(json.contains("\"runtime\""));
        assert!(json.contains("\"agents\""));
        assert!(json.contains("\"approvals\""));
        assert!(json.contains("\"budget\""));
    }
}
