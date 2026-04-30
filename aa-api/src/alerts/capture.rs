//! Background task that captures budget alerts from the broadcast channel.

use std::sync::Arc;

use tokio::sync::broadcast;

use aa_gateway::budget::types::BudgetAlert;

use super::AlertStore;

/// Spawn a background task that subscribes to the budget alert broadcast
/// channel and records each alert into the given store.
///
/// The task runs until the broadcast channel is closed (all senders dropped).
/// `RecvError::Lagged` is handled gracefully by logging and continuing.
pub fn spawn_alert_capture(
    mut rx: broadcast::Receiver<BudgetAlert>,
    store: Arc<dyn AlertStore>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(alert) => {
                    store.record(&alert);
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(
                        count,
                        "alert capture task lagged behind broadcast, skipped {count} alerts"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("budget alert broadcast channel closed, stopping capture task");
                    break;
                }
            }
        }
    })
}
