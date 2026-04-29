//! Fire-and-forget webhook delivery loop.
//!
//! Subscribes to the [`ApprovalRequest`] broadcast from the approval queue and
//! the [`BudgetAlert`] broadcast from the budget tracker, converts each event
//! into a JSON envelope, and POSTs it to the configured webhook URL.
//!
//! Failures are logged at `warn` level but never block the source channels.

use tokio::sync::broadcast;

use aa_runtime::approval::ApprovalRequest;

use super::publisher;
use super::webhook::WebhookTarget;
use crate::budget::BudgetAlert;

// TODO(AAASM-75): Add DataLeakEvent match arm when EnrichedEvent::DataLeak variant lands.

/// Run the webhook delivery loop until both broadcast channels close.
///
/// This function is intended to be spawned as a background tokio task.
/// It returns only when both senders are dropped (runtime shutdown).
pub async fn webhook_delivery_loop(
    target: WebhookTarget,
    mut approval_rx: broadcast::Receiver<ApprovalRequest>,
    mut budget_rx: broadcast::Receiver<BudgetAlert>,
) {
    tracing::info!(url = %target.url(), "webhook delivery loop started");

    loop {
        tokio::select! {
            result = approval_rx.recv() => {
                match result {
                    Ok(request) => {
                        let envelope = publisher::approval_to_envelope(&request);
                        if let Err(e) = target.deliver(&envelope).await {
                            tracing::warn!(
                                error = %e,
                                request_id = %request.request_id,
                                "failed to deliver approval webhook"
                            );
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            dropped = n,
                            "approval event receiver lagged, some events were not delivered"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("approval broadcast channel closed");
                        break;
                    }
                }
            }
            result = budget_rx.recv() => {
                match result {
                    Ok(alert) => {
                        let envelope = publisher::budget_alert_to_envelope(&alert);
                        if let Err(e) = target.deliver(&envelope).await {
                            tracing::warn!(
                                error = %e,
                                threshold_pct = alert.threshold_pct,
                                "failed to deliver budget alert webhook"
                            );
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            dropped = n,
                            "budget alert receiver lagged, some events were not delivered"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("budget alert broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("webhook delivery loop stopped");
}
