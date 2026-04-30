//! Rendering functions for `aasm status` output.

use super::models::RuntimeHealth;

/// Render the Runtime Health section to stdout.
pub fn render_runtime_health(health: &RuntimeHealth) {
    println!("RUNTIME HEALTH");
    println!("──────────────");
    let indicator = if health.reachable { "✓" } else { "✗" };
    println!("  API:    {indicator} {}", health.status);
    println!();
}
