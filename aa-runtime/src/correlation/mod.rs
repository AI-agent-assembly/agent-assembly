//! Intent-Action Causal Correlation engine.
//!
//! Matches LLM response intents (captured via SDK or proxy) to kernel-level
//! actions (captured via eBPF) using PID lineage and a configurable time
//! window. Detects intent→action divergence and unauthorized escalation.
//!
//! Inspired by the AgentSight paper.

pub mod config;
pub mod engine;
pub mod event;
pub mod outcome;
pub mod pid;
pub mod window;

pub use config::CorrelationConfig;
pub use engine::CorrelationEngine;
pub use event::{ActionEvent, CorrelationEvent, IntentEvent};
pub use outcome::{CausalCorrelation, CorrelationOutcome};
pub use pid::PidLineage;
pub use window::SlidingWindow;

use aa_proto::assembly::audit::v1::audit_event::Detail;
use aa_proto::assembly::common::v1::ActionType;
use uuid::Uuid;

use crate::pipeline::event::{EnrichedEvent, EventSource};

/// Convert an [`EnrichedEvent`] from the pipeline broadcast channel into a
/// [`CorrelationEvent`] for ingestion by the correlation engine.
///
/// # Mapping rules
///
/// | Source   | ActionType                                    | Result          |
/// |----------|-----------------------------------------------|-----------------|
/// | SDK      | `LLM_CALL`, `TOOL_CALL`                       | `Intent`        |
/// | eBPF     | `FILE_OPERATION`, `NETWORK_CALL`, `PROCESS_EXEC` | `Action`     |
/// | *        | anything else                                 | `None`          |
///
/// Returns `None` for events that do not participate in causal correlation
/// (e.g., policy violations, layer degradation, or proxy-sourced events that
/// don't yet have a mapping).
pub fn try_from_enriched(event: &EnrichedEvent) -> Option<CorrelationEvent> {
    let action_type = ActionType::try_from(event.inner.action_type).ok()?;
    let event_id = event.inner.event_id.parse::<Uuid>().unwrap_or_else(|_| Uuid::new_v4());
    let timestamp_ms = event.received_at_ms as u64;
    // TODO(AAASM-150): Proto does not carry PID — use 0 as placeholder until
    // the AuditEvent schema is extended with a `pid` field.
    let pid: u32 = 0;

    match (&event.source, action_type) {
        // SDK-sourced LLM and tool calls → intents
        (EventSource::Sdk, ActionType::LlmCall) | (EventSource::Sdk, ActionType::ToolCall) => {
            let (intent_text, action_keyword) = extract_intent_fields(&event.inner.detail, action_type);
            Some(CorrelationEvent::Intent(IntentEvent {
                event_id,
                timestamp_ms,
                pid,
                intent_text,
                action_keyword,
            }))
        }
        // eBPF-sourced syscall actions → actions
        (EventSource::EBpf, ActionType::FileOperation)
        | (EventSource::EBpf, ActionType::NetworkCall)
        | (EventSource::EBpf, ActionType::ProcessExec) => {
            let (syscall, details) = extract_action_fields(&event.inner.detail, action_type);
            Some(CorrelationEvent::Action(ActionEvent {
                event_id,
                timestamp_ms,
                pid,
                syscall,
                details,
            }))
        }
        _ => None,
    }
}

/// Extract intent-side fields from the audit event detail payload.
fn extract_intent_fields(detail: &Option<Detail>, action_type: ActionType) -> (String, String) {
    let action_keyword = action_type.as_str_name().to_string();
    let intent_text = match detail {
        Some(Detail::LlmCall(d)) => format!("model={} provider={}", d.model, d.provider),
        Some(Detail::ToolCall(d)) => format!("tool={} source={}", d.tool_name, d.tool_source),
        _ => String::new(),
    };
    (intent_text, action_keyword)
}

/// Extract action-side fields from the audit event detail payload.
fn extract_action_fields(detail: &Option<Detail>, action_type: ActionType) -> (String, String) {
    match detail {
        Some(Detail::FileOp(d)) => (d.operation.clone(), d.path.clone()),
        Some(Detail::Network(d)) => (d.protocol.clone(), format!("{}:{}", d.host, d.port)),
        Some(Detail::Process(d)) => (d.command.clone(), d.args.join(" ")),
        _ => (action_type.as_str_name().to_string(), String::new()),
    }
}
