//! Dashboard application state and panel definitions.

use std::collections::VecDeque;

use crate::commands::status::models::{
    AgentRow, ApprovalResponse, ApprovalsSummary, BudgetRow, RuntimeHealth,
};

use super::dialog::DialogAction;

/// Maximum number of events retained in the scrollback buffer.
pub const EVENT_LOG_CAPACITY: usize = 200;

/// Which panel is currently focused / highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Fleet health + agents table (top-left).
    Agents,
    /// Real-time event log (top-right).
    EventLog,
    /// Pending approvals list (bottom-left).
    Approvals,
    /// Budget / cost gauge (bottom-right).
    Budget,
}

impl Panel {
    /// Cycle to the next panel in Tab order.
    pub fn next(self) -> Self {
        match self {
            Self::Agents => Self::EventLog,
            Self::EventLog => Self::Approvals,
            Self::Approvals => Self::Budget,
            Self::Budget => Self::Agents,
        }
    }

    /// Cycle to the previous panel in Shift+Tab order.
    pub fn prev(self) -> Self {
        match self {
            Self::Agents => Self::Budget,
            Self::EventLog => Self::Agents,
            Self::Approvals => Self::EventLog,
            Self::Budget => Self::Approvals,
        }
    }
}

/// A single event line received from the WebSocket stream.
#[derive(Debug, Clone)]
pub struct EventEntry {
    pub timestamp: String,
    pub event_type: String,
    pub agent_id: String,
    pub message: String,
}

/// Full application state for the TUI dashboard.
#[derive(Debug)]
pub struct DashboardState {
    /// Which panel currently has focus.
    pub active_panel: Panel,

    /// Runtime health summary.
    pub runtime: RuntimeHealth,
    /// Agent rows for the agents table.
    pub agents: Vec<AgentRow>,
    /// Aggregated approvals summary (count + oldest age).
    pub approvals_summary: ApprovalsSummary,
    /// Individual pending approval requests (for the approvals list).
    pub pending_approvals: Vec<ApprovalResponse>,
    /// Budget / cost summary.
    pub budget: BudgetRow,

    /// Ring buffer of recent governance events from the WebSocket.
    pub event_log: VecDeque<EventEntry>,

    /// Scroll offset in the event log panel.
    pub event_log_scroll: u16,
    /// Selected index in the pending approvals list.
    pub approval_selected: usize,

    /// Whether the help overlay is currently visible.
    pub show_help: bool,
    /// The pending confirm dialog action, if any.
    pub confirm_dialog: Option<DialogAction>,

    /// Whether the dashboard should quit on the next tick.
    pub should_quit: bool,
}

impl DashboardState {
    /// Create an initial empty state with defaults.
    pub fn new() -> Self {
        Self {
            active_panel: Panel::Agents,
            runtime: RuntimeHealth {
                reachable: false,
                status: "connecting…".to_string(),
                uptime_secs: 0,
                active_connections: 0,
                pipeline_lag_ms: 0,
            },
            agents: Vec::new(),
            approvals_summary: ApprovalsSummary {
                pending_count: 0,
                oldest_pending_age: None,
            },
            pending_approvals: Vec::new(),
            budget: BudgetRow {
                daily_spend_usd: "0.00".to_string(),
                monthly_spend_usd: None,
                daily_limit_usd: None,
                monthly_limit_usd: None,
                date: String::new(),
                per_agent: vec![],
            },
            event_log: VecDeque::with_capacity(EVENT_LOG_CAPACITY),
            event_log_scroll: 0,
            approval_selected: 0,
            show_help: false,
            confirm_dialog: None,
            should_quit: false,
        }
    }

    /// Append an event to the log ring buffer, evicting the oldest if full.
    pub fn push_event(&mut self, entry: EventEntry) {
        if self.event_log.len() >= EVENT_LOG_CAPACITY {
            self.event_log.pop_front();
        }
        self.event_log.push_back(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_next_cycles_through_all() {
        let start = Panel::Agents;
        let second = start.next();
        assert_eq!(second, Panel::EventLog);
        let third = second.next();
        assert_eq!(third, Panel::Approvals);
        let fourth = third.next();
        assert_eq!(fourth, Panel::Budget);
        let back = fourth.next();
        assert_eq!(back, Panel::Agents);
    }

    #[test]
    fn panel_prev_cycles_backwards() {
        let start = Panel::Agents;
        let last = start.prev();
        assert_eq!(last, Panel::Budget);
        let third = last.prev();
        assert_eq!(third, Panel::Approvals);
        let second = third.prev();
        assert_eq!(second, Panel::EventLog);
        let first = second.prev();
        assert_eq!(first, Panel::Agents);
    }

    #[test]
    fn new_state_defaults() {
        let state = DashboardState::new();
        assert_eq!(state.active_panel, Panel::Agents);
        assert!(!state.runtime.reachable);
        assert!(state.agents.is_empty());
        assert_eq!(state.approvals_summary.pending_count, 0);
        assert!(state.pending_approvals.is_empty());
        assert!(!state.show_help);
        assert!(state.confirm_dialog.is_none());
        assert!(!state.should_quit);
    }

    #[test]
    fn push_event_within_capacity() {
        let mut state = DashboardState::new();
        state.push_event(EventEntry {
            timestamp: "2026-04-30T10:00:00Z".to_string(),
            event_type: "violation".to_string(),
            agent_id: "a1".to_string(),
            message: "test".to_string(),
        });
        assert_eq!(state.event_log.len(), 1);
    }

    #[test]
    fn push_event_evicts_oldest_at_capacity() {
        let mut state = DashboardState::new();
        for i in 0..EVENT_LOG_CAPACITY {
            state.push_event(EventEntry {
                timestamp: format!("t{i}"),
                event_type: "test".to_string(),
                agent_id: "a1".to_string(),
                message: format!("msg {i}"),
            });
        }
        assert_eq!(state.event_log.len(), EVENT_LOG_CAPACITY);
        // Push one more — oldest should be evicted.
        state.push_event(EventEntry {
            timestamp: "overflow".to_string(),
            event_type: "test".to_string(),
            agent_id: "a1".to_string(),
            message: "overflow".to_string(),
        });
        assert_eq!(state.event_log.len(), EVENT_LOG_CAPACITY);
        assert_eq!(state.event_log.front().unwrap().timestamp, "t1");
        assert_eq!(state.event_log.back().unwrap().timestamp, "overflow");
    }
}
