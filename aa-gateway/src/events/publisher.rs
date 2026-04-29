//! Converts runtime events into JSON-serializable webhook payloads.
//!
//! Proto-generated types lack `serde::Serialize`, so this module manually
//! constructs a [`serde_json::Value`] representation of each
//! [`EnvelopedEvent`](aa_proto::assembly::event::v1::EnvelopedEvent) payload.

use aa_proto::assembly::common::v1 as common;
use aa_proto::assembly::event::v1::ApprovalRequested;
use aa_runtime::pipeline::event::EnrichedEvent;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::budget::BudgetAlert;

/// Event type routing keys used in the envelope's `event_type` field.
pub const EVENT_TYPE_APPROVAL_REQUESTED: &str = "approval.requested";
pub const EVENT_TYPE_BUDGET_THRESHOLD: &str = "budget.threshold_hit";

/// Convert an [`EnrichedEvent`] whose inner `AuditEvent` triggered an
/// approval-hold into a JSON envelope for webhook delivery.
///
/// Returns `None` if the event does not contain an approval request.
pub fn approval_to_envelope(event: &EnrichedEvent, approval: &ApprovalRequested) -> Value {
    let event_id = Uuid::now_v7().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();

    json!({
        "event_id": event_id,
        "event_type": EVENT_TYPE_APPROVAL_REQUESTED,
        "published_at": { "unix_ms": now_ms },
        "source": "aa-gateway",
        "payload": {
            "approval_request": {
                "approval_id": approval.approval_id,
                "agent_id": agent_id_to_json(approval.agent_id.as_ref()),
                "action_summary": approval.action_summary,
                "action_context_json": String::from_utf8_lossy(&approval.action_context_json).to_string(),
                "expires_at_unix_ms": approval.expires_at_unix_ms,
                "notify_user_ids": approval.notify_user_ids,
            }
        },
        "enrichment": {
            "received_at_ms": event.received_at_ms,
            "agent_id": event.agent_id,
            "sequence_number": event.sequence_number,
        }
    })
}

/// Convert a [`BudgetAlert`] into a JSON envelope for webhook delivery.
pub fn budget_alert_to_envelope(alert: &BudgetAlert) -> Value {
    let event_id = Uuid::now_v7().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let agent_bytes = alert.agent_id.as_bytes();
    let agent_uuid = Uuid::from_bytes(*agent_bytes);

    json!({
        "event_id": event_id,
        "event_type": EVENT_TYPE_BUDGET_THRESHOLD,
        "published_at": { "unix_ms": now_ms },
        "source": "aa-gateway",
        "payload": {
            "budget_alert": {
                "agent_id": agent_uuid.to_string(),
                "current_spend": alert.spent_usd,
                "budget_limit": alert.limit_usd,
                "percent_used": alert.threshold_pct,
            }
        }
    })
}

/// Serialize a proto [`AgentId`](common::AgentId) to JSON, handling `None`.
fn agent_id_to_json(agent_id: Option<&common::AgentId>) -> Value {
    match agent_id {
        Some(id) => json!({
            "org_id": id.org_id,
            "team_id": id.team_id,
            "agent_id": id.agent_id,
        }),
        None => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_core::AgentId;
    use aa_proto::assembly::audit::v1::AuditEvent;
    use aa_runtime::pipeline::event::{EnrichedEvent, EventSource};

    fn sample_enriched_event() -> EnrichedEvent {
        EnrichedEvent {
            inner: AuditEvent::default(),
            received_at_ms: 1700000000000,
            source: EventSource::Sdk,
            agent_id: "test-agent".to_string(),
            connection_id: 1,
            sequence_number: 42,
        }
    }

    #[test]
    fn approval_envelope_has_correct_event_type() {
        let event = sample_enriched_event();
        let approval = ApprovalRequested {
            approval_id: "apr-001".to_string(),
            agent_id: Some(common::AgentId {
                org_id: "org".to_string(),
                team_id: "team".to_string(),
                agent_id: "agent".to_string(),
            }),
            action_summary: "delete production database".to_string(),
            action_context_json: b"{}".to_vec(),
            expires_at_unix_ms: 1700000060000,
            notify_user_ids: vec!["user-1".to_string()],
        };

        let envelope = approval_to_envelope(&event, &approval);
        assert_eq!(envelope["event_type"], "approval.requested");
        assert_eq!(envelope["source"], "aa-gateway");
        assert_eq!(
            envelope["payload"]["approval_request"]["approval_id"],
            "apr-001"
        );
        assert_eq!(
            envelope["payload"]["approval_request"]["action_summary"],
            "delete production database"
        );
        assert_eq!(envelope["enrichment"]["sequence_number"], 42);
    }

    #[test]
    fn approval_envelope_has_uuid_v7_event_id() {
        let event = sample_enriched_event();
        let approval = ApprovalRequested {
            approval_id: "apr-002".to_string(),
            ..Default::default()
        };

        let envelope = approval_to_envelope(&event, &approval);
        let id_str = envelope["event_id"].as_str().unwrap();
        // UUID v7 parses successfully and has version 7
        let parsed = Uuid::parse_str(id_str).expect("valid UUID");
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn budget_alert_envelope_has_correct_fields() {
        let alert = BudgetAlert {
            agent_id: AgentId::from_bytes([1; 16]),
            threshold_pct: 80,
            spent_usd: 80.0,
            limit_usd: 100.0,
        };

        let envelope = budget_alert_to_envelope(&alert);
        assert_eq!(envelope["event_type"], "budget.threshold_hit");
        assert_eq!(envelope["source"], "aa-gateway");
        assert_eq!(envelope["payload"]["budget_alert"]["current_spend"], 80.0);
        assert_eq!(envelope["payload"]["budget_alert"]["budget_limit"], 100.0);
        assert_eq!(envelope["payload"]["budget_alert"]["percent_used"], 80);
    }

    #[test]
    fn budget_alert_envelope_has_uuid_v7_event_id() {
        let alert = BudgetAlert {
            agent_id: AgentId::from_bytes([2; 16]),
            threshold_pct: 95,
            spent_usd: 95.0,
            limit_usd: 100.0,
        };

        let envelope = budget_alert_to_envelope(&alert);
        let id_str = envelope["event_id"].as_str().unwrap();
        let parsed = Uuid::parse_str(id_str).expect("valid UUID");
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn agent_id_to_json_handles_none() {
        let result = agent_id_to_json(None);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn agent_id_to_json_handles_some() {
        let id = common::AgentId {
            org_id: "o".to_string(),
            team_id: "t".to_string(),
            agent_id: "a".to_string(),
        };
        let result = agent_id_to_json(Some(&id));
        assert_eq!(result["org_id"], "o");
        assert_eq!(result["team_id"], "t");
        assert_eq!(result["agent_id"], "a");
    }
}
