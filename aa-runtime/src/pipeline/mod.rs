//! Event aggregation pipeline — receives IpcFrames, enriches, batches, and fans out.

pub mod event;
pub mod metrics;

pub use event::{EnrichedEvent, EventSource};
pub use metrics::PipelineMetrics;

use crate::config::RuntimeConfig;
use crate::ipc::IpcFrame;
use aa_proto::assembly::audit::v1::{audit_event::Detail, AuditEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

/// Configuration for the event aggregation pipeline.
///
/// Derived from [`RuntimeConfig`] via [`PipelineConfig::from_runtime_config`].
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Depth of the mpsc inbound channel.
    pub input_buffer: usize,
    /// Maximum events in a batch before an early flush.
    pub batch_size: usize,
    /// Interval between scheduled batch flushes.
    pub flush_interval: Duration,
    /// Capacity of the broadcast ring buffer.
    pub broadcast_capacity: usize,
    /// Agent identity — copied from `RuntimeConfig::agent_id`.
    pub agent_id: String,
}

impl PipelineConfig {
    /// Build a [`PipelineConfig`] from a [`RuntimeConfig`].
    pub fn from_runtime_config(c: &RuntimeConfig) -> Self {
        Self {
            input_buffer:       c.pipeline_input_buffer,
            batch_size:         c.pipeline_batch_size,
            flush_interval:     Duration::from_millis(c.pipeline_flush_interval_ms),
            broadcast_capacity: c.pipeline_broadcast_capacity,
            agent_id:           c.agent_id.clone(),
        }
    }
}

/// Start the event aggregation pipeline.
///
/// Consumes `rx` (the inbound IpcFrame channel from the IPC server),
/// enriches and batches events, and fans them out via `broadcast_tx`.
///
/// Returns when `token` is cancelled — flushing any pending batch first.
pub async fn run(
    mut rx:       mpsc::Receiver<IpcFrame>,
    broadcast_tx: broadcast::Sender<EnrichedEvent>,
    config:       PipelineConfig,
    metrics:      Arc<PipelineMetrics>,
    token:        CancellationToken,
) {
    let mut batch: Vec<EnrichedEvent> = Vec::with_capacity(config.batch_size);
    let mut ticker = tokio::time::interval(config.flush_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;

            _ = token.cancelled() => {
                // Drain any remaining batch before exiting.
                if !batch.is_empty() {
                    flush(&mut batch, &broadcast_tx, &metrics);
                }
                break;
            }

            Some(frame) = rx.recv() => {
                if let IpcFrame::EventReport(event) = frame {
                    let enriched = enrich(event, &config.agent_id);
                    metrics.record_processed(1);
                    if is_policy_violation(&enriched) {
                        // Bypass the batch — emit immediately.
                        let _ = broadcast_tx.send(enriched);
                    } else {
                        batch.push(enriched);
                        if batch.len() >= config.batch_size {
                            flush(&mut batch, &broadcast_tx, &metrics);
                        }
                    }
                }
                // PolicyQuery, ApprovalResponse, Heartbeat: not pipeline events, ignored.
            }

            _ = ticker.tick() => {
                if !batch.is_empty() {
                    flush(&mut batch, &broadcast_tx, &metrics);
                }
            }
        }
    }
    tracing::info!("pipeline task stopped");
}

/// Enrich a raw [`AuditEvent`] with runtime-side metadata.
fn enrich(event: AuditEvent, agent_id: &str) -> EnrichedEvent {
    use std::time::{SystemTime, UNIX_EPOCH};
    let received_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    EnrichedEvent {
        inner: event,
        received_at_ms,
        source: EventSource::Sdk,
        agent_id: agent_id.to_string(),
    }
}

/// Returns `true` if this event's detail is a `PolicyViolation`.
///
/// Policy violation events bypass batching and are emitted immediately.
fn is_policy_violation(event: &EnrichedEvent) -> bool {
    matches!(event.inner.detail, Some(Detail::Violation(_)))
}

/// Broadcast all events in `batch` and record metrics.
///
/// Clears `batch` after broadcasting. Errors from `broadcast_tx.send`
/// (all receivers dropped) are silently ignored — the pipeline does not
/// require any active subscribers to operate.
fn flush(
    batch:        &mut Vec<EnrichedEvent>,
    broadcast_tx: &broadcast::Sender<EnrichedEvent>,
    metrics:      &PipelineMetrics,
) {
    let n = batch.len() as u64;
    for event in batch.drain(..) {
        let _ = broadcast_tx.send(event);
    }
    metrics.record_batch_size(n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_proto::assembly::audit::v1::{audit_event::Detail, AuditEvent, PolicyViolation};

    fn make_audit_event() -> AuditEvent {
        AuditEvent::default()
    }

    fn make_policy_violation_event() -> AuditEvent {
        AuditEvent {
            detail: Some(Detail::Violation(PolicyViolation {
                policy_rule: "test-rule".to_string(),
                blocked_action: "test-action".to_string(),
                reason: "test-reason".to_string(),
            })),
            ..Default::default()
        }
    }

    #[test]
    fn enrich_sets_agent_id() {
        let event = make_audit_event();
        let enriched = enrich(event, "my-agent");
        assert_eq!(enriched.agent_id, "my-agent");
    }

    #[test]
    fn enrich_sets_received_at_ms_positive() {
        let event = make_audit_event();
        let enriched = enrich(event, "agent");
        assert!(enriched.received_at_ms > 0);
    }

    #[test]
    fn enrich_sets_source_to_sdk() {
        let event = make_audit_event();
        let enriched = enrich(event, "agent");
        assert_eq!(enriched.source, EventSource::Sdk);
    }

    #[test]
    fn is_policy_violation_true_for_violation_detail() {
        let event = make_policy_violation_event();
        let enriched = enrich(event, "agent");
        assert!(is_policy_violation(&enriched));
    }

    #[test]
    fn is_policy_violation_false_for_normal_event() {
        let event = make_audit_event(); // detail = None
        let enriched = enrich(event, "agent");
        assert!(!is_policy_violation(&enriched));
    }

    #[test]
    fn flush_empty_batch_does_nothing() {
        let (tx, _rx) = broadcast::channel::<EnrichedEvent>(16);
        let metrics = PipelineMetrics::default();
        let mut batch: Vec<EnrichedEvent> = vec![];
        flush(&mut batch, &tx, &metrics);
        assert_eq!(metrics.last_batch_size(), 0);
        assert_eq!(metrics.processed(), 0);
    }

    #[test]
    fn flush_broadcasts_all_events_and_records_batch_size() {
        let (tx, mut rx) = broadcast::channel::<EnrichedEvent>(16);
        let metrics = PipelineMetrics::default();
        let mut batch = vec![
            enrich(make_audit_event(), "a"),
            enrich(make_audit_event(), "b"),
        ];
        flush(&mut batch, &tx, &metrics);
        assert!(batch.is_empty());
        assert_eq!(metrics.last_batch_size(), 2);
        // Both events were sent and are receivable.
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn from_runtime_config_copies_all_fields() {
        let runtime_config = RuntimeConfig {
            agent_id: "test-agent".to_string(),
            worker_threads: 0,
            shutdown_timeout_secs: 30,
            ipc_max_connections: 64,
            pipeline_input_buffer: 5_000,
            pipeline_batch_size: 50,
            pipeline_flush_interval_ms: 200,
            pipeline_broadcast_capacity: 512,
        };

        let pipeline_config = PipelineConfig::from_runtime_config(&runtime_config);

        assert_eq!(pipeline_config.input_buffer, runtime_config.pipeline_input_buffer);
        assert_eq!(pipeline_config.batch_size, runtime_config.pipeline_batch_size);
        assert_eq!(
            pipeline_config.flush_interval,
            Duration::from_millis(runtime_config.pipeline_flush_interval_ms)
        );
        assert_eq!(
            pipeline_config.broadcast_capacity,
            runtime_config.pipeline_broadcast_capacity
        );
        assert_eq!(pipeline_config.agent_id, runtime_config.agent_id);
    }

    #[test]
    fn pipeline_config_is_clone() {
        let pipeline_config = PipelineConfig {
            input_buffer: 5_000,
            batch_size: 50,
            flush_interval: Duration::from_millis(200),
            broadcast_capacity: 512,
            agent_id: "test-agent".to_string(),
        };

        let cloned = pipeline_config.clone();

        assert_eq!(cloned.agent_id, pipeline_config.agent_id);
    }
}
