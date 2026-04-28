//! Event aggregation pipeline — receives IpcFrames, enriches, batches, and fans out.

pub mod event;
pub mod metrics;

pub use event::{EnrichedEvent, EventSource};
pub use metrics::PipelineMetrics;

use crate::config::RuntimeConfig;
use crate::ipc::{IpcFrame, IpcResponse, ResponseRouter};
use crate::policy::PolicyRules;
use aa_proto::assembly::audit::v1::{audit_event::Detail, AuditEvent, PolicyViolation};
use aa_proto::assembly::common::v1::ActionType;
use std::sync::atomic::{AtomicU64, Ordering};
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
            input_buffer: c.pipeline_input_buffer,
            batch_size: c.pipeline_batch_size,
            flush_interval: Duration::from_millis(c.pipeline_flush_interval_ms),
            broadcast_capacity: c.pipeline_broadcast_capacity,
            agent_id: c.agent_id.clone(),
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
    mut rx: mpsc::Receiver<(u64, IpcFrame)>,
    broadcast_tx: broadcast::Sender<EnrichedEvent>,
    config: PipelineConfig,
    metrics: Arc<PipelineMetrics>,
    token: CancellationToken,
    policy: Arc<PolicyRules>,
    response_router: ResponseRouter,
) {
    let seq = AtomicU64::new(0);
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

            Some((connection_id, frame)) = rx.recv() => {
                if let IpcFrame::EventReport(event) = frame {
                    let enriched = enrich(event, &config.agent_id, connection_id, &seq);
                    tracing::debug!(sequence_number = enriched.sequence_number, connection_id, "event enriched");
                    metrics.record_processed(1);
                    ::metrics::counter!("aa_events_received_total").increment(1);
                    if is_policy_violation(&enriched, &policy) {
                        // Bypass the batch — emit immediately.
                        ::metrics::counter!("aa_policy_violations_total").increment(1);
                        // Push a ViolationAlert back to the originating SDK connection.
                        push_violation_alert(&enriched, &response_router).await;
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
fn enrich(event: AuditEvent, agent_id: &str, connection_id: u64, seq: &AtomicU64) -> EnrichedEvent {
    use std::time::{SystemTime, UNIX_EPOCH};
    let received_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    let sequence_number = seq.fetch_add(1, Ordering::Relaxed);
    EnrichedEvent {
        inner: event,
        received_at_ms,
        source: EventSource::Sdk,
        agent_id: agent_id.to_string(),
        connection_id,
        sequence_number,
    }
}

/// Returns `true` if this event should bypass batching and be emitted immediately.
///
/// An event is a violation if either:
/// - Its detail is a `PolicyViolation` proto message, or
/// - Any rule in `policy.rules` has a `blocked_actions` entry that matches the
///   event's `action_type` (compared as the proto enum's string name).
fn is_policy_violation(event: &EnrichedEvent, policy: &PolicyRules) -> bool {
    if matches!(event.inner.detail, Some(Detail::Violation(_))) {
        return true;
    }
    let action_str = ActionType::try_from(event.inner.action_type)
        .map(|a| a.as_str_name())
        .unwrap_or("");
    for rule in &policy.rules {
        if rule.blocked_actions.iter().any(|ba| ba == action_str) {
            tracing::warn!(
                rule = %rule.name,
                action = %action_str,
                "policy rule matched — event bypassing batch"
            );
            return true;
        }
    }
    false
}

/// Extract a `PolicyViolation` from an `EnrichedEvent`, if one is present.
///
/// Returns `Some` when the event's detail is `Detail::Violation(_)`.
/// Returns `None` for rule-matched events that have no embedded violation proto —
/// in that case the SDK already knows the action was blocked.
fn extract_violation(event: &EnrichedEvent) -> Option<PolicyViolation> {
    match &event.inner.detail {
        Some(Detail::Violation(v)) => Some(v.clone()),
        _ => None,
    }
}

/// Send a `ViolationAlert` to the SDK connection that originated `event`.
///
/// Looks up the per-connection sender in the `ResponseRouter`. If the connection
/// has already disconnected the entry will be absent and the alert is silently
/// dropped — the connection is gone so there is no point delivering it.
async fn push_violation_alert(event: &EnrichedEvent, router: &crate::ipc::ResponseRouter) {
    let Some(violation) = extract_violation(event) else {
        // Rule-matched events don't carry a PolicyViolation proto; skip.
        return;
    };
    let sender = {
        let map = router.read().await;
        map.get(&event.connection_id).cloned()
    };
    if let Some(tx) = sender {
        if tx.send(IpcResponse::ViolationAlert(violation)).await.is_err() {
            tracing::debug!(
                connection_id = event.connection_id,
                "ViolationAlert dropped — connection already closed"
            );
        }
    }
}

/// Broadcast all events in `batch` and record metrics.
///
/// Clears `batch` after broadcasting. Errors from `broadcast_tx.send`
/// (all receivers dropped) are silently ignored — the pipeline does not
/// require any active subscribers to operate.
fn flush(batch: &mut Vec<EnrichedEvent>, broadcast_tx: &broadcast::Sender<EnrichedEvent>, metrics: &PipelineMetrics) {
    let n = batch.len() as u64;
    for event in batch.drain(..) {
        let _ = broadcast_tx.send(event);
    }
    ::metrics::counter!("aa_events_emitted_total").increment(n);
    metrics.record_batch_size(n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PolicyRules;
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
        let seq = AtomicU64::new(0);
        let enriched = enrich(event, "my-agent", 0, &seq);
        assert_eq!(enriched.agent_id, "my-agent");
    }

    #[test]
    fn enrich_sets_received_at_ms_positive() {
        let event = make_audit_event();
        let seq = AtomicU64::new(0);
        let enriched = enrich(event, "agent", 0, &seq);
        assert!(enriched.received_at_ms > 0);
    }

    #[test]
    fn enrich_sets_source_to_sdk() {
        let event = make_audit_event();
        let seq = AtomicU64::new(0);
        let enriched = enrich(event, "agent", 0, &seq);
        assert_eq!(enriched.source, EventSource::Sdk);
    }

    #[test]
    fn is_policy_violation_true_for_violation_detail() {
        let event = make_policy_violation_event();
        let seq = AtomicU64::new(0);
        let enriched = enrich(event, "agent", 0, &seq);
        assert!(is_policy_violation(&enriched, &PolicyRules::default()));
    }

    #[test]
    fn is_policy_violation_false_for_normal_event() {
        let event = make_audit_event(); // detail = None
        let seq = AtomicU64::new(0);
        let enriched = enrich(event, "agent", 0, &seq);
        assert!(!is_policy_violation(&enriched, &PolicyRules::default()));
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
        let seq = AtomicU64::new(0);
        let mut batch = vec![
            enrich(make_audit_event(), "a", 0, &seq),
            enrich(make_audit_event(), "b", 0, &seq),
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
            metrics_addr: "0.0.0.0:8080".to_string(),
            policy_path: None,
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

    // -----------------------------------------------------------------------
    // Integration test helpers
    // -----------------------------------------------------------------------

    fn test_config(batch_size: usize, flush_interval_ms: u64) -> PipelineConfig {
        PipelineConfig {
            input_buffer: 1_024,
            batch_size,
            flush_interval: Duration::from_millis(flush_interval_ms),
            broadcast_capacity: 1_024,
            agent_id: "test-agent".to_string(),
        }
    }

    fn normal_event() -> AuditEvent {
        AuditEvent::default()
    }

    fn violation_event() -> AuditEvent {
        AuditEvent {
            detail: Some(Detail::Violation(PolicyViolation {
                policy_rule: "rule".to_string(),
                blocked_action: "action".to_string(),
                reason: "reason".to_string(),
            })),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Integration tests — spin up a real run() task
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn batch_flushes_on_size_threshold() {
        let config = test_config(3, 10_000); // batch_size=3, very long interval (won't fire)
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Send 3 events — batch threshold reached, should flush before interval
        for _ in 0..3 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }

        // All 3 events should arrive within a short time
        for _ in 0..3 {
            tokio::time::timeout(Duration::from_millis(500), broadcast_rx.recv())
                .await
                .expect("timed out waiting for event")
                .expect("broadcast error");
        }
        assert_eq!(metrics.processed(), 3);
        token.cancel();
    }

    #[tokio::test]
    async fn batch_flushes_on_interval() {
        let config = test_config(100, 50); // batch_size=100 (won't reach), interval=50ms
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Send 5 events (less than batch_size=100) — should arrive after interval flush
        for _ in 0..5 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }

        for _ in 0..5 {
            tokio::time::timeout(Duration::from_millis(500), broadcast_rx.recv())
                .await
                .expect("timed out waiting for event from interval flush")
                .expect("broadcast error");
        }
        assert_eq!(metrics.processed(), 5);
        token.cancel();
    }

    #[tokio::test]
    async fn policy_violation_bypasses_batch() {
        // batch_size=100, very long interval — only a violation should arrive
        let config = test_config(100, 10_000);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Send a violation — should arrive immediately, bypassing batch
        tx.send((0, IpcFrame::EventReport(violation_event()))).await.unwrap();

        let event = tokio::time::timeout(Duration::from_millis(200), broadcast_rx.recv())
            .await
            .expect("violation event should arrive immediately, before any flush interval")
            .expect("broadcast error");

        assert!(matches!(event.inner.detail, Some(Detail::Violation(_))));
        assert_eq!(metrics.processed(), 1);
        token.cancel();
    }

    #[tokio::test]
    async fn cancellation_flushes_pending_batch() {
        let config = test_config(100, 10_000); // large batch, long interval
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        let handle = tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Send 5 events (batch won't flush yet)
        for _ in 0..5 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }

        // Wait until the run loop has processed all 5 events before cancelling,
        // so they are guaranteed to be in the pending batch when the flush fires.
        let deadline = std::time::Instant::now() + Duration::from_millis(200);
        loop {
            if metrics.processed() == 5 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "events were not processed within 200ms"
            );
            tokio::task::yield_now().await;
        }
        token.cancel();

        // Wait for pipeline to stop
        tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("pipeline did not stop after cancellation")
            .expect("pipeline task panicked");

        // All 5 events should be in the broadcast channel
        let mut received = 0;
        while broadcast_rx.try_recv().is_ok() {
            received += 1;
        }
        assert_eq!(received, 5, "expected 5 events flushed on cancellation");
    }

    #[tokio::test]
    async fn non_event_frames_ignored() {
        let config = test_config(100, 50);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, _broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Send non-event frames
        tx.send((0, IpcFrame::Heartbeat)).await.unwrap();

        // Give run loop a moment to process
        tokio::time::sleep(Duration::from_millis(20)).await;

        // No events processed
        assert_eq!(metrics.processed(), 0);
        token.cancel();
    }

    #[tokio::test]
    async fn rule_match_bypasses_batch() {
        use crate::policy::{PolicyRule, PolicyRules};
        use aa_proto::assembly::common::v1::ActionType;

        // Create a policy that blocks FILE_OPERATION
        let policy = std::sync::Arc::new(PolicyRules {
            rules: vec![PolicyRule {
                name: "block-files".to_string(),
                blocked_actions: vec![ActionType::FileOperation.as_str_name().to_string()],
            }],
        });

        // batch_size=100, very long interval — only a rule-matched event should arrive immediately
        let config = test_config(100, 10_000);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            policy,
            crate::ipc::new_response_router(),
        ));

        // Build an AuditEvent with action_type = FILE_OPERATION
        let event = AuditEvent {
            action_type: ActionType::FileOperation as i32,
            ..Default::default()
        };
        tx.send((0, IpcFrame::EventReport(event))).await.unwrap();

        // Should arrive immediately (before flush interval)
        let received = tokio::time::timeout(Duration::from_millis(200), broadcast_rx.recv())
            .await
            .expect("rule-matched event should bypass batch and arrive immediately")
            .expect("broadcast error");

        assert_eq!(received.source, EventSource::Sdk);
        assert_eq!(metrics.processed(), 1);
        token.cancel();
    }

    #[tokio::test]
    async fn non_matching_action_stays_in_batch() {
        use crate::policy::{PolicyRule, PolicyRules};
        use aa_proto::assembly::common::v1::ActionType;

        // Policy only blocks FILE_OPERATION
        let policy = std::sync::Arc::new(PolicyRules {
            rules: vec![PolicyRule {
                name: "block-files".to_string(),
                blocked_actions: vec![ActionType::FileOperation.as_str_name().to_string()],
            }],
        });

        // batch_size=100, very long interval — event should NOT arrive before timeout
        let config = test_config(100, 10_000);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            policy,
            crate::ipc::new_response_router(),
        ));

        // Yield briefly so the pipeline's interval fires its immediate first tick
        // (tokio::time::interval ticks once immediately on creation).
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Build a TOOL_CALL event — not blocked by the policy
        let event = AuditEvent {
            action_type: ActionType::ToolCall as i32,
            ..Default::default()
        };
        tx.send((0, IpcFrame::EventReport(event))).await.unwrap();

        // Should NOT arrive before the flush interval (100ms timeout)
        let result = tokio::time::timeout(Duration::from_millis(100), broadcast_rx.recv()).await;
        assert!(
            result.is_err(),
            "non-matching event should stay in batch, not arrive immediately"
        );

        token.cancel();
    }

    #[tokio::test]
    async fn sequence_numbers_are_consecutive_within_a_batch() {
        // batch_size=3 so we get a single flush of 3 events and can check ordering.
        let config = test_config(3, 10_000);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        for _ in 0..3 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }

        let mut seq_numbers = Vec::new();
        for _ in 0..3 {
            let event = tokio::time::timeout(Duration::from_millis(500), broadcast_rx.recv())
                .await
                .expect("timed out waiting for event")
                .expect("broadcast error");
            seq_numbers.push(event.sequence_number);
        }

        // Sequence numbers must be strictly monotonically increasing, starting at 0.
        assert_eq!(
            seq_numbers,
            vec![0, 1, 2],
            "expected consecutive sequence numbers 0, 1, 2"
        );
        token.cancel();
    }

    #[tokio::test]
    async fn sequence_numbers_are_monotonic_across_batches() {
        // Two separate batch flushes — sequence counter must not reset between them.
        let config = test_config(2, 10_000); // batch_size=2
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(64);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(64);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // First batch of 2
        for _ in 0..2 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }
        let first_batch: Vec<u64> = {
            let mut v = Vec::new();
            for _ in 0..2 {
                let e = tokio::time::timeout(Duration::from_millis(500), broadcast_rx.recv())
                    .await
                    .expect("timed out waiting for first batch")
                    .expect("broadcast error");
                v.push(e.sequence_number);
            }
            v
        };

        // Second batch of 2
        for _ in 0..2 {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }
        let second_batch: Vec<u64> = {
            let mut v = Vec::new();
            for _ in 0..2 {
                let e = tokio::time::timeout(Duration::from_millis(500), broadcast_rx.recv())
                    .await
                    .expect("timed out waiting for second batch")
                    .expect("broadcast error");
                v.push(e.sequence_number);
            }
            v
        };

        assert_eq!(first_batch, vec![0, 1]);
        assert_eq!(
            second_batch,
            vec![2, 3],
            "sequence counter must not reset between batches"
        );
        token.cancel();
    }

    #[tokio::test]
    #[ignore]
    async fn pipeline_load_benchmark() {
        // Run with: cargo test -p aa-runtime -- --ignored pipeline_load_benchmark --nocapture
        const EVENT_COUNT: u64 = 100_000;

        let config = test_config(100, 10);
        let (tx, rx) = mpsc::channel::<(u64, IpcFrame)>(10_000);
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel::<EnrichedEvent>(10_000);
        let metrics = Arc::new(PipelineMetrics::default());
        let token = CancellationToken::new();

        tokio::spawn(run(
            rx,
            broadcast_tx,
            config,
            metrics.clone(),
            token.clone(),
            Arc::new(PolicyRules::default()),
            crate::ipc::new_response_router(),
        ));

        // Spawn a receiver that drains the broadcast channel
        tokio::spawn(async move { while broadcast_rx.recv().await.is_ok() {} });

        let start = std::time::Instant::now();

        for _ in 0..EVENT_COUNT {
            tx.send((0, IpcFrame::EventReport(normal_event()))).await.unwrap();
        }

        // Wait until all events are processed
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if metrics.processed() >= EVENT_COUNT {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!(
                    "load benchmark timeout: only {} / {} events processed",
                    metrics.processed(),
                    EVENT_COUNT
                );
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let elapsed = start.elapsed();
        println!(
            "pipeline_load_benchmark: {} events in {:?} ({:.0} events/sec)",
            EVENT_COUNT,
            elapsed,
            EVENT_COUNT as f64 / elapsed.as_secs_f64()
        );

        assert!(elapsed.as_secs() < 5, "100k events took more than 5s: {:?}", elapsed);
        token.cancel();
    }
}
