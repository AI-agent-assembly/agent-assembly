//! Shared application state for the Axum server.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use aa_gateway::budget::tracker::BudgetTracker;
use aa_gateway::engine::PolicyEngine;
use aa_runtime::approval::ApprovalQueue;

use crate::events::EventBroadcast;
use crate::replay::ReplayBuffer;

/// Shared state available to all Axum handlers via `Extension<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// Policy engine for governance decisions.
    pub policy_engine: Arc<PolicyEngine>,
    /// Cost tracking and budget enforcement.
    pub budget_tracker: Arc<BudgetTracker>,
    /// Human-in-the-loop approval request queue.
    pub approval_queue: Arc<ApprovalQueue>,
    /// Unified event broadcast bus for streaming to clients.
    pub events: Arc<EventBroadcast>,
    /// Circular replay buffer for reconnecting WebSocket clients.
    pub replay_buffer: ReplayBuffer,
    /// Monotonic counter for assigning GovernanceEvent ids.
    pub next_event_id: Arc<AtomicU64>,
}
