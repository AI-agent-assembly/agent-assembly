//! TUI rendering — draws the 4-panel dashboard layout.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table};
use ratatui::Frame;

use super::state::{DashboardState, Panel};

/// Render the entire dashboard UI to the terminal frame.
pub fn draw(f: &mut Frame, state: &DashboardState) {
    let size = f.area();

    // Split vertically: top half and bottom half, plus a 1-line footer.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            Constraint::Min(1),
        ])
        .split(size);

    // Top: Agents (left) | Event Log (right)
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(outer[0]);

    // Bottom: Approvals (left) | Budget (right)
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(outer[1]);

    draw_agents_panel(f, top[0], state);
    draw_event_log_panel(f, top[1], state);
    draw_approvals_panel(f, bottom[0], state);
    draw_budget_panel(f, bottom[1], state);
    draw_footer(f, outer[2], state);
}

/// Build a Block with a highlighted border when the panel is focused.
fn panel_block(title: &str, panel: Panel, state: &DashboardState) -> Block<'static> {
    let is_active = state.active_panel == panel;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_style)
}

/// Top-left: runtime health header + agents table.
fn draw_agents_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = panel_block("Agents", Panel::Agents, state);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Reserve 2 lines for the health header.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    // Health header line.
    let status_indicator = if state.runtime.reachable { "●" } else { "○" };
    let status_color = if state.runtime.reachable {
        Color::Green
    } else {
        Color::Red
    };
    let uptime = format_duration(state.runtime.uptime_secs);
    let header_line = Line::from(vec![
        Span::styled(
            format!("{status_indicator} "),
            Style::default().fg(status_color),
        ),
        Span::raw(format!(
            "{} | up {} | {} conns | lag {}ms",
            state.runtime.status,
            uptime,
            state.runtime.active_connections,
            state.runtime.pipeline_lag_ms,
        )),
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // Agents table.
    let header = Row::new(vec!["ID", "NAME", "STATUS", "FW", "SESS", "VIOL", "LAYER"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    let rows: Vec<Row> = state
        .agents
        .iter()
        .map(|a| {
            let status_style = match a.status.as_str() {
                "Running" | "Active" => Style::default().fg(Color::Green),
                "Error" | "Failed" => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::Yellow),
            };
            Row::new(vec![
                Cell::from(truncate(&a.id, 8)),
                Cell::from(a.name.as_str()),
                Cell::from(a.status.as_str()).style(status_style),
                Cell::from(a.framework.as_str()),
                Cell::from(a.sessions.to_string()),
                Cell::from(a.violations_today.to_string()),
                Cell::from(a.layer.as_str()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(9),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
        ],
    )
    .header(header);

    f.render_widget(table, chunks[1]);
}

/// Top-right: scrollable event log from the WebSocket stream.
fn draw_event_log_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = panel_block("Event Log", Panel::EventLog, state);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = state
        .event_log
        .iter()
        .rev()
        .map(|e| {
            let type_color = match e.event_type.as_str() {
                "violation" => Color::Red,
                "approval" => Color::Yellow,
                "budget" => Color::Magenta,
                _ => Color::White,
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", short_timestamp(&e.timestamp)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<10} ", e.event_type),
                    Style::default().fg(type_color),
                ),
                Span::raw(&e.message),
            ]))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

/// Bottom-left: pending approval requests with selection highlight.
fn draw_approvals_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let title = format!(
        "Approvals ({} pending)",
        state.approvals_summary.pending_count
    );
    let block = panel_block(&title, Panel::Approvals, state);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.pending_approvals.is_empty() {
        let msg = Paragraph::new("No pending approvals")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<ListItem> = state
        .pending_approvals
        .iter()
        .enumerate()
        .map(|(i, ap)| {
            let style = if i == state.approval_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", short_timestamp(&ap.created_at)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(format!(
                    "{} — {} ({})",
                    ap.agent_id, ap.action, ap.reason
                )),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

/// Bottom-right: budget gauge and cost breakdown.
fn draw_budget_panel(f: &mut Frame, area: Rect, state: &DashboardState) {
    let block = panel_block("Budget", Panel::Budget, state);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    // Daily spend gauge.
    let (ratio, label) = compute_budget_ratio(state);
    let gauge_color = if ratio > 0.9 {
        Color::Red
    } else if ratio > 0.7 {
        Color::Yellow
    } else {
        Color::Green
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(label);
    f.render_widget(gauge, chunks[0]);

    // Per-agent cost breakdown.
    let items: Vec<ListItem> = state
        .budget
        .per_agent
        .iter()
        .map(|entry| {
            ListItem::new(format!(
                "  {} — ${}",
                entry.agent_id, entry.daily_spend_usd
            ))
        })
        .collect();
    let list = List::new(items);
    f.render_widget(list, chunks[1]);
}

/// Footer bar with keyboard shortcuts.
fn draw_footer(f: &mut Frame, area: Rect, _state: &DashboardState) {
    let footer = Line::from(vec![
        Span::styled(" Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" panel  "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" scroll  "),
        Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("/"),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" approve/reject  "),
        Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" help  "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit"),
    ]);
    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

/// Compute the budget gauge ratio and label string.
fn compute_budget_ratio(state: &DashboardState) -> (f64, String) {
    let spend: f64 = state
        .budget
        .daily_spend_usd
        .parse()
        .unwrap_or(0.0);

    let limit: Option<f64> = state
        .budget
        .daily_limit_usd
        .as_deref()
        .and_then(|s| s.parse().ok());

    match limit {
        Some(lim) if lim > 0.0 => {
            let ratio = (spend / lim).min(1.0);
            (ratio, format!("${spend:.2} / ${lim:.2}"))
        }
        _ => (0.0, format!("${spend:.2} (no limit set)")),
    }
}

/// Truncate a string to `max` characters, appending "…" if shortened.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Extract HH:MM:SS from an ISO timestamp for compact display.
fn short_timestamp(ts: &str) -> &str {
    // "2026-04-30T10:00:00Z" → "10:00:00"
    if ts.len() >= 19 {
        &ts[11..19]
    } else {
        ts
    }
}

/// Format seconds into a human-readable duration string.
fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("abc", 5), "abc");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc…");
    }

    #[test]
    fn short_timestamp_extracts_time() {
        assert_eq!(short_timestamp("2026-04-30T10:15:30Z"), "10:15:30");
    }

    #[test]
    fn short_timestamp_returns_input_if_too_short() {
        assert_eq!(short_timestamp("short"), "short");
    }

    #[test]
    fn format_duration_hours_and_minutes() {
        assert_eq!(format_duration(3661), "1h 1m");
    }

    #[test]
    fn format_duration_minutes_only() {
        assert_eq!(format_duration(300), "5m");
    }

    #[test]
    fn compute_budget_ratio_with_limit() {
        let state = DashboardState::new();
        let mut state = state;
        state.budget.daily_spend_usd = "50.00".to_string();
        state.budget.daily_limit_usd = Some("100.00".to_string());
        let (ratio, label) = compute_budget_ratio(&state);
        assert!((ratio - 0.5).abs() < 0.01);
        assert!(label.contains("50.00"));
        assert!(label.contains("100.00"));
    }

    #[test]
    fn compute_budget_ratio_no_limit() {
        let state = DashboardState::new();
        let (ratio, label) = compute_budget_ratio(&state);
        assert!((ratio - 0.0).abs() < 0.01);
        assert!(label.contains("no limit"));
    }

    #[test]
    fn compute_budget_ratio_capped_at_one() {
        let mut state = DashboardState::new();
        state.budget.daily_spend_usd = "150.00".to_string();
        state.budget.daily_limit_usd = Some("100.00".to_string());
        let (ratio, _) = compute_budget_ratio(&state);
        assert!((ratio - 1.0).abs() < 0.01);
    }
}
