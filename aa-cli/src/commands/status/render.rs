//! Rendering functions for `aasm status` output.

use comfy_table::{ContentArrangement, Table};

use super::models::{AgentRow, RuntimeHealth};

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
