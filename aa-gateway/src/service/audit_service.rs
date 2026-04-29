//! `AuditService` tonic trait implementation wiring gRPC RPCs to [`AuditWriter`].
//!
//! [`AuditWriter`]: crate::audit::AuditWriter

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use aa_core::identity::{AgentId, SessionId};
use aa_core::{AuditEntry, AuditEventType};
use aa_proto::assembly::audit::v1::audit_service_server::AuditService;
use aa_proto::assembly::audit::v1::{AuditEvent, ReportEventsRequest, ReportEventsResponse, StreamEventsResponse};
use aa_proto::assembly::common::v1::Decision;

use crate::service::convert;

/// gRPC service implementation wiring `ReportEvents` / `StreamEvents` to the
/// audit writer channel.
pub struct AuditServiceImpl {
    audit_tx: mpsc::Sender<AuditEntry>,
    audit_drops: Arc<AtomicU64>,
    seq: AtomicU64,
}

impl AuditServiceImpl {
    /// Create a new service backed by the given audit channel.
    pub fn new(audit_tx: mpsc::Sender<AuditEntry>, audit_drops: Arc<AtomicU64>) -> Self {
        Self {
            audit_tx,
            audit_drops,
            seq: AtomicU64::new(0),
        }
    }

    /// Convert a proto `AuditEvent` into a core `AuditEntry` and send via try_send.
    ///
    /// Returns the event_id on success, or an empty string if the entry was dropped.
    fn ingest_event(&self, event: &AuditEvent, previous_hash: [u8; 32]) -> String {
        let event_id = event.event_id.clone();
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);

        let agent_id = event
            .agent_id
            .as_ref()
            .map(|a| AgentId::from_bytes(convert::hash_to_16(&a.agent_id)))
            .unwrap_or_else(|| AgentId::from_bytes([0u8; 16]));

        let session_id = if event.trace_id.is_empty() {
            SessionId::from_bytes([0u8; 16])
        } else {
            SessionId::from_bytes(convert::hash_to_16(&event.trace_id))
        };

        let timestamp_ns = event
            .occurred_at
            .as_ref()
            .map(|t| (t.unix_ms as u64).saturating_mul(1_000_000))
            .unwrap_or(0);

        let event_type = decision_to_audit_event_type(event.decision);

        let payload = serde_json::json!({
            "event_id": &event.event_id,
            "action_type": event.action_type,
            "span_id": &event.span_id,
            "parent_span_id": &event.parent_span_id,
        })
        .to_string();

        let entry = AuditEntry::new(
            seq,
            timestamp_ns,
            event_type,
            agent_id,
            session_id,
            payload,
            previous_hash,
        );

        if let Err(e) = self.audit_tx.try_send(entry) {
            match e {
                mpsc::error::TrySendError::Full(_) => {
                    tracing::warn!(seq, "audit channel full — event dropped");
                    self.audit_drops.fetch_add(1, Ordering::Relaxed);
                }
                mpsc::error::TrySendError::Closed(_) => {
                    tracing::error!("audit channel closed — AuditWriter task has exited");
                }
            }
        }

        event_id
    }
}

/// Map a proto `Decision` i32 to `AuditEventType`.
fn decision_to_audit_event_type(decision: i32) -> AuditEventType {
    match Decision::try_from(decision) {
        Ok(Decision::Allow) => AuditEventType::ToolCallIntercepted,
        Ok(Decision::Deny) => AuditEventType::PolicyViolation,
        Ok(Decision::Redact) => AuditEventType::CredentialLeakBlocked,
        Ok(Decision::Pending) => AuditEventType::ApprovalRequested,
        _ => AuditEventType::PolicyViolation,
    }
}

#[tonic::async_trait]
impl AuditService for AuditServiceImpl {
    async fn report_events(
        &self,
        request: Request<ReportEventsRequest>,
    ) -> Result<Response<ReportEventsResponse>, Status> {
        let batch = request.into_inner();
        let mut event_ids = Vec::with_capacity(batch.events.len());

        for event in &batch.events {
            let id = self.ingest_event(event, [0u8; 32]);
            event_ids.push(id);
        }

        Ok(Response::new(ReportEventsResponse { event_ids }))
    }

    async fn stream_events(
        &self,
        request: Request<tonic::Streaming<AuditEvent>>,
    ) -> Result<Response<StreamEventsResponse>, Status> {
        let mut stream = request.into_inner();
        let mut events_received: i64 = 0;

        while let Some(event) = stream.message().await.map_err(|e| {
            tracing::error!(error = %e, "stream_events receive error");
            Status::internal(format!("stream receive error: {e}"))
        })? {
            self.ingest_event(&event, [0u8; 32]);
            events_received += 1;
        }

        Ok(Response::new(StreamEventsResponse { events_received }))
    }
}
