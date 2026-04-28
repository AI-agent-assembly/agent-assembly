//! Proto ↔ core type conversions for the PolicyService gRPC layer.
//!
//! Bridges the structural gap between protobuf message types
//! (`CheckActionRequest`, `CheckActionResponse`) and the core domain types
//! (`AgentContext`, `GovernanceAction`, `PolicyResult`).

use aa_core::identity::{AgentId, SessionId};
use aa_core::time::Timestamp;
use aa_core::{AgentContext, FileMode, GovernanceAction, PolicyResult};
use aa_proto::assembly::common::v1::Decision;
use aa_proto::assembly::policy::v1::action_context::Action;
use aa_proto::assembly::policy::v1::{CheckActionRequest, CheckActionResponse, RedactInstructions, RedactRule};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::engine::EvaluationResult;

/// Errors arising from malformed or incomplete proto requests.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The `agent_id` field is missing from the request.
    #[error("missing agent_id")]
    MissingAgentId,
    /// The `context` oneof field is missing or empty.
    #[error("missing action context")]
    MissingContext,
    /// The file operation string is not one of "read", "write", "append", "delete".
    #[error("unknown file operation: {0}")]
    UnknownFileOp(String),
}

/// Hash a string into a 16-byte identifier using SHA-256 truncation.
///
/// Proto identity fields are variable-length strings; core identity types are
/// fixed `[u8; 16]`. This deterministic mapping avoids collisions in practice
/// while satisfying the type constraint.
fn hash_to_16(s: &str) -> [u8; 16] {
    let digest = Sha256::digest(s.as_bytes());
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

/// Convert a [`CheckActionRequest`] into the core domain pair
/// ([`AgentContext`], [`GovernanceAction`]).
pub fn request_to_core(req: &CheckActionRequest) -> Result<(AgentContext, GovernanceAction), ConvertError> {
    // --- Agent context ---
    let proto_agent = req.agent_id.as_ref().ok_or(ConvertError::MissingAgentId)?;
    let agent_id = AgentId::from_bytes(hash_to_16(&proto_agent.agent_id));
    let session_id = SessionId::from_bytes(hash_to_16(&req.trace_id));

    let mut metadata = BTreeMap::new();
    if !proto_agent.org_id.is_empty() {
        metadata.insert("org_id".into(), proto_agent.org_id.clone());
    }
    if !proto_agent.team_id.is_empty() {
        metadata.insert("team_id".into(), proto_agent.team_id.clone());
    }
    if !req.credential_token.is_empty() {
        metadata.insert("credential_token".into(), req.credential_token.clone());
    }
    if !req.span_id.is_empty() {
        metadata.insert("span_id".into(), req.span_id.clone());
    }

    let ctx = AgentContext {
        agent_id,
        session_id,
        pid: 0, // not available in proto — set to 0
        started_at: Timestamp::from_nanos(0),
        metadata,
    };

    // --- Governance action ---
    let context = req.context.as_ref().ok_or(ConvertError::MissingContext)?;
    let action_oneof = context.action.as_ref().ok_or(ConvertError::MissingContext)?;

    let action = match action_oneof {
        Action::ToolCall(tc) => GovernanceAction::ToolCall {
            name: tc.tool_name.clone(),
            args: String::from_utf8_lossy(&tc.args_json).into_owned(),
        },
        Action::FileOp(fo) => {
            let mode = match fo.operation.as_str() {
                "read" => FileMode::Read,
                "write" | "create" => FileMode::Write,
                "append" => FileMode::Append,
                "delete" => FileMode::Delete,
                other => return Err(ConvertError::UnknownFileOp(other.to_string())),
            };
            GovernanceAction::FileAccess {
                path: fo.path.clone(),
                mode,
            }
        }
        Action::NetworkCall(nc) => {
            let url = format!("{}://{}:{}", nc.protocol, nc.host, nc.port);
            GovernanceAction::NetworkRequest {
                url,
                method: "CONNECT".into(),
            }
        }
        Action::ProcessExec(pe) => {
            let command = if pe.args.is_empty() {
                pe.command.clone()
            } else {
                format!("{} {}", pe.command, pe.args.join(" "))
            };
            GovernanceAction::ProcessExec { command }
        }
        Action::LlmCall(lc) => {
            let args = serde_json::json!({
                "model": lc.model,
                "prompt_tokens": lc.prompt_tokens,
                "contains_pii": lc.contains_pii,
            })
            .to_string();
            GovernanceAction::ToolCall {
                name: "llm_call".into(),
                args,
            }
        }
    };

    Ok((ctx, action))
}

/// Convert an [`EvaluationResult`] into a [`CheckActionResponse`].
///
/// `latency_us` is the measured evaluation wall time in microseconds.
/// `policy_rule` is the identifier of the rule that triggered (empty for Allow).
///
/// When the engine detected credential/PII findings and produced a redacted payload,
/// the response uses `Decision::Redact` with the redacted field paths.
pub fn eval_result_to_response(eval: &EvaluationResult, latency_us: i64, policy_rule: &str) -> CheckActionResponse {
    // If the scanner produced redaction findings, return REDACT with instructions.
    if !eval.credential_findings.is_empty() {
        let rules: Vec<RedactRule> = eval
            .credential_findings
            .iter()
            .map(|f| RedactRule {
                field_path: format!("$.{:?}", f.kind),
                replacement: "[REDACTED]".into(),
            })
            .collect();
        return CheckActionResponse {
            decision: Decision::Redact as i32,
            reason: "sensitive data detected".into(),
            policy_rule: "data_pattern_scan".into(),
            approval_id: String::new(),
            redact: Some(RedactInstructions { rules }),
            decision_latency_us: latency_us,
        };
    }

    match &eval.decision {
        PolicyResult::Allow => CheckActionResponse {
            decision: Decision::Allow as i32,
            reason: String::new(),
            policy_rule: String::new(),
            approval_id: String::new(),
            redact: None,
            decision_latency_us: latency_us,
        },
        PolicyResult::Deny { reason } => CheckActionResponse {
            decision: Decision::Deny as i32,
            reason: reason.clone(),
            policy_rule: policy_rule.to_string(),
            approval_id: String::new(),
            redact: None,
            decision_latency_us: latency_us,
        },
        PolicyResult::RequiresApproval { .. } => CheckActionResponse {
            decision: Decision::Pending as i32,
            reason: "human approval required".into(),
            policy_rule: policy_rule.to_string(),
            approval_id: uuid::Uuid::new_v4().to_string(),
            redact: None,
            decision_latency_us: latency_us,
        },
    }
}

/// Convert a [`PolicyResult`] into a [`CheckActionResponse`].
///
/// Convenience wrapper for tests and callers that only have a `PolicyResult`
/// (no redaction findings).
pub fn result_to_response(result: &PolicyResult, latency_us: i64, policy_rule: &str) -> CheckActionResponse {
    let eval = EvaluationResult {
        decision: result.clone(),
        redacted_payload: None,
        credential_findings: Vec::new(),
    };
    eval_result_to_response(&eval, latency_us, policy_rule)
}
