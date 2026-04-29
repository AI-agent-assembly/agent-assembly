//! Unified event broadcast bus for the API layer.
//!
//! Aggregates the individual `tokio::sync::broadcast` channels from the
//! runtime, gateway, and proxy crates into a single struct that the API
//! layer (and downstream WebSocket streaming) can subscribe to.

use aa_gateway::budget::types::BudgetAlert;
use aa_runtime::approval::ApprovalRequest;
use aa_runtime::pipeline::event::PipelineEvent;
use tokio::sync::broadcast;

/// Default channel capacity for each event broadcast.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Unified event broadcast bus.
///
/// Holds one `broadcast::Sender` per event domain so that API consumers
/// (e.g. the WebSocket streaming endpoint) can subscribe to any
/// combination without reaching into individual subsystem internals.
pub struct EventBroadcast {
    pipeline_tx: broadcast::Sender<PipelineEvent>,
    approval_tx: broadcast::Sender<ApprovalRequest>,
    budget_tx: broadcast::Sender<BudgetAlert>,
}

impl EventBroadcast {
    /// Create a new `EventBroadcast` with the given per-channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (pipeline_tx, _) = broadcast::channel(capacity);
        let (approval_tx, _) = broadcast::channel(capacity);
        let (budget_tx, _) = broadcast::channel(capacity);
        Self {
            pipeline_tx,
            approval_tx,
            budget_tx,
        }
    }

    /// Subscribe to pipeline audit events.
    pub fn subscribe_pipeline(&self) -> broadcast::Receiver<PipelineEvent> {
        self.pipeline_tx.subscribe()
    }

    /// Subscribe to human-approval request events.
    pub fn subscribe_approvals(&self) -> broadcast::Receiver<ApprovalRequest> {
        self.approval_tx.subscribe()
    }

    /// Subscribe to budget threshold alerts.
    pub fn subscribe_budget(&self) -> broadcast::Receiver<BudgetAlert> {
        self.budget_tx.subscribe()
    }

    /// Get a clone of the pipeline event sender.
    pub fn pipeline_sender(&self) -> broadcast::Sender<PipelineEvent> {
        self.pipeline_tx.clone()
    }

    /// Get a clone of the approval event sender.
    pub fn approval_sender(&self) -> broadcast::Sender<ApprovalRequest> {
        self.approval_tx.clone()
    }

    /// Get a clone of the budget alert sender.
    pub fn budget_sender(&self) -> broadcast::Sender<BudgetAlert> {
        self.budget_tx.clone()
    }
}

impl Default for EventBroadcast {
    fn default() -> Self {
        Self::new(DEFAULT_CHANNEL_CAPACITY)
    }
}
