//! Webhook delivery startup wiring.
//!
//! Reads `AA_WEBHOOK_URL` from the environment and, if set, subscribes to the
//! approval and budget broadcast channels and spawns the delivery loop.

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use aa_runtime::approval::ApprovalRequest;

use super::delivery::webhook_delivery_loop;
use super::webhook::WebhookTarget;
use crate::budget::BudgetAlert;

/// Environment variable name for the webhook URL.
pub const WEBHOOK_URL_ENV: &str = "AA_WEBHOOK_URL";

/// Optionally spawn the webhook delivery loop.
///
/// Reads `AA_WEBHOOK_URL` from the environment. If set, creates a shared
/// [`reqwest::Client`], subscribes to both broadcast channels, and spawns the
/// delivery loop as a background tokio task.
///
/// If the variable is unset or empty, logs an INFO message and returns `None`.
pub fn maybe_spawn_webhook(
    approval_queue: &Arc<aa_runtime::approval::ApprovalQueue>,
    budget_alert_rx: broadcast::Receiver<BudgetAlert>,
) -> Option<JoinHandle<()>> {
    let url = match std::env::var(WEBHOOK_URL_ENV) {
        Ok(url) if !url.is_empty() => url,
        _ => {
            tracing::info!(
                env = WEBHOOK_URL_ENV,
                "webhook URL not configured, event notifications disabled"
            );
            return None;
        }
    };

    tracing::info!(url = %url, "webhook delivery enabled");

    let client = reqwest::Client::new();
    let target = WebhookTarget::new(client, url);
    let approval_rx: broadcast::Receiver<ApprovalRequest> = approval_queue.subscribe_events();

    let handle = tokio::spawn(webhook_delivery_loop(target, approval_rx, budget_alert_rx));
    Some(handle)
}
