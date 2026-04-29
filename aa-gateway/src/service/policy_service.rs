//! `PolicyService` tonic trait implementation wiring gRPC RPCs to `PolicyEngine`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use aa_core::identity::{AgentId, SessionId};
use aa_core::time::Timestamp;
use aa_core::{AuditEntry, AuditEventType};
use aa_proto::assembly::policy::v1::policy_service_server::PolicyService;
use aa_proto::assembly::policy::v1::{BatchCheckRequest, BatchCheckResponse, CheckActionRequest, CheckActionResponse};

use crate::engine::PolicyEngine;
use crate::service::convert;

/// gRPC service implementation wiring `CheckAction` / `BatchCheck` to [`PolicyEngine`].
pub struct PolicyServiceImpl {
    engine: Arc<PolicyEngine>,
    audit_tx: mpsc::Sender<AuditEntry>,
    audit_drops: Arc<AtomicU64>,
}

impl PolicyServiceImpl {
    /// Create a new service backed by the given policy engine and audit channel.
    pub fn new(
        engine: Arc<PolicyEngine>,
        audit_tx: mpsc::Sender<AuditEntry>,
        audit_drops: Arc<AtomicU64>,
    ) -> Self {
        Self { engine, audit_tx, audit_drops }
    }

    /// Evaluate a single request against the engine, returning the gRPC response.
    #[allow(clippy::result_large_err)] // tonic::Status is the standard gRPC error type
    fn evaluate_one(&self, req: &CheckActionRequest) -> Result<CheckActionResponse, Status> {
        let (ctx, action) = convert::request_to_core(req).map_err(|e| {
            tracing::error!(error = %e, "failed to convert CheckActionRequest");
            Status::invalid_argument(e.to_string())
        })?;

        let start = Instant::now();
        let eval = self.engine.evaluate(&ctx, &action);
        let latency_us = start.elapsed().as_micros() as i64;

        // Derive a policy_rule label from the deny/approval reason.
        let policy_rule = match &eval.decision {
            aa_core::PolicyResult::Allow => "",
            aa_core::PolicyResult::Deny { reason } => reason.as_str(),
            aa_core::PolicyResult::RequiresApproval { .. } => "requires_approval",
        };

        Ok(convert::eval_result_to_response(&eval, latency_us, policy_rule))
    }

    /// Build an `AuditEntry` from a request and evaluation result, then fire-and-forget
    /// via `try_send`. Never blocks the caller.
    fn record_audit(
        &self,
        req: &CheckActionRequest,
        response: &CheckActionResponse,
        seq: u64,
        previous_hash: [u8; 32],
    ) {
        let proto_agent = match req.agent_id.as_ref() {
            Some(a) => a,
            None => return, // No agent identity — cannot construct entry.
        };
        let agent_id = AgentId::from_bytes(convert::hash_to_16(&proto_agent.agent_id));
        let session_id = SessionId::from_bytes(convert::hash_to_16(&req.trace_id));
        let event_type = Self::decision_to_event_type_from_response(response.decision);
        let timestamp_ns = Timestamp::from(SystemTime::now()).as_nanos();

        let payload = serde_json::json!({
            "action_type": req.action_type,
            "decision": response.decision,
            "reason": &response.reason,
            "policy_rule": &response.policy_rule,
            "latency_us": response.decision_latency_us,
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
                    tracing::warn!(seq, "audit channel full — entry dropped");
                    self.audit_drops.fetch_add(1, Ordering::Relaxed);
                }
                mpsc::error::TrySendError::Closed(_) => {
                    tracing::error!("audit channel closed — AuditWriter task has exited");
                }
            }
        }
    }

    /// Map a proto `Decision` i32 to `AuditEventType`.
    fn decision_to_event_type_from_response(decision: i32) -> AuditEventType {
        use aa_proto::assembly::common::v1::Decision;
        match Decision::try_from(decision) {
            Ok(Decision::Allow) => AuditEventType::ToolCallIntercepted,
            Ok(Decision::Deny) => AuditEventType::PolicyViolation,
            Ok(Decision::Redact) => AuditEventType::CredentialLeakBlocked,
            Ok(Decision::Pending) => AuditEventType::ApprovalRequested,
            _ => AuditEventType::PolicyViolation, // fallback for unknown
        }
    }
}

#[tonic::async_trait]
impl PolicyService for PolicyServiceImpl {
    async fn check_action(
        &self,
        request: Request<CheckActionRequest>,
    ) -> Result<Response<CheckActionResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!(
            agent_id = ?req.agent_id.as_ref().map(|a| &a.agent_id),
            action_type = req.action_type,
            trace_id = %req.trace_id,
            "check_action request"
        );

        let response = self.evaluate_one(&req)?;

        tracing::debug!(
            decision = response.decision,
            latency_us = response.decision_latency_us,
            "check_action response"
        );

        if response.decision != aa_proto::assembly::common::v1::Decision::Allow as i32 {
            tracing::warn!(
                decision = response.decision,
                reason = %response.reason,
                policy_rule = %response.policy_rule,
                "non-allow decision"
            );
        }

        // Fire-and-forget audit entry — never blocks the response.
        self.record_audit(&req, &response, 0, [0u8; 32]);

        Ok(Response::new(response))
    }

    async fn batch_check(&self, request: Request<BatchCheckRequest>) -> Result<Response<BatchCheckResponse>, Status> {
        let batch = request.into_inner();
        let mut responses = Vec::with_capacity(batch.requests.len());

        for (i, req) in batch.requests.iter().enumerate() {
            let resp = self.evaluate_one(req)?;
            self.record_audit(req, &resp, i as u64, [0u8; 32]);
            responses.push(resp);
        }

        Ok(Response::new(BatchCheckResponse { responses }))
    }
}
