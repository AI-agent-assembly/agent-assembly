//! `PolicyService` tonic trait implementation wiring gRPC RPCs to `PolicyEngine`.

use std::sync::Arc;
use std::time::Instant;

use tonic::{Request, Response, Status};

use aa_proto::assembly::policy::v1::policy_service_server::PolicyService;
use aa_proto::assembly::policy::v1::{BatchCheckRequest, BatchCheckResponse, CheckActionRequest, CheckActionResponse};

use crate::engine::PolicyEngine;
use crate::service::convert;

/// gRPC service implementation wiring `CheckAction` / `BatchCheck` to [`PolicyEngine`].
pub struct PolicyServiceImpl {
    engine: Arc<PolicyEngine>,
}

impl PolicyServiceImpl {
    /// Create a new service backed by the given policy engine.
    pub fn new(engine: Arc<PolicyEngine>) -> Self {
        Self { engine }
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

        Ok(Response::new(response))
    }

    async fn batch_check(&self, request: Request<BatchCheckRequest>) -> Result<Response<BatchCheckResponse>, Status> {
        let batch = request.into_inner();
        let mut responses = Vec::with_capacity(batch.requests.len());

        for req in &batch.requests {
            responses.push(self.evaluate_one(req)?);
        }

        Ok(Response::new(BatchCheckResponse { responses }))
    }
}
