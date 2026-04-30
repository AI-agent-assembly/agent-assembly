//! Rendering functions for `aasm status` output.

use comfy_table::{ContentArrangement, Table};

use super::models::{AgentRow, ApprovalsSummary, RuntimeHealth};

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
