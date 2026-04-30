//! Shared application state for the Axum server.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use aa_gateway::budget::tracker::BudgetTracker;
use aa_gateway::engine::PolicyEngine;
use aa_gateway::policy::history::PolicyHistoryStore;
use aa_gateway::registry::AgentRegistry;
use aa_runtime::approval::ApprovalQueue;

use crate::auth::api_key::ApiKeyStore;
use crate::auth::config::AuthConfig;
use crate::auth::jwt::{JwtSigner, JwtVerifier};
use crate::auth::rate_limit::RateLimiter;
use crate::events::EventBroadcast;
use crate::replay::ReplayBuffer;

/// Shared state available to all Axum handlers via `Extension<AppState>`.
#[derive(Clone)]
pub struct AppState {
    /// Agent registry for tracking active agents.
    pub agent_registry: Arc<AgentRegistry>,
    /// Policy engine for governance decisions.
    pub policy_engine: Arc<PolicyEngine>,
    /// Cost tracking and budget enforcement.
    pub budget_tracker: Arc<BudgetTracker>,
    /// Human-in-the-loop approval request queue.
    pub approval_queue: Arc<ApprovalQueue>,
    /// Policy version history store.
    pub policy_history: Arc<dyn PolicyHistoryStore>,
    /// Unified event broadcast bus for streaming to clients.
    pub events: Arc<EventBroadcast>,
    /// Circular replay buffer for reconnecting WebSocket clients.
    pub replay_buffer: ReplayBuffer,
    /// Monotonic counter for assigning GovernanceEvent ids.
    pub next_event_id: Arc<AtomicU64>,
    /// Authentication configuration.
    pub auth_config: Arc<AuthConfig>,
    /// Loaded API key entries for validation.
    pub key_store: Arc<ApiKeyStore>,
    /// Per-key rate limiter.
    pub rate_limiter: Arc<RateLimiter>,
    /// JWT token signer.
    pub jwt_signer: Arc<JwtSigner>,
    /// JWT token verifier.
    pub jwt_verifier: Arc<JwtVerifier>,
}
