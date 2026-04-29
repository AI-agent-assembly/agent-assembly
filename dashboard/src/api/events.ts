/**
 * WebSocket event payload types.
 *
 * Manually curated because OpenAPI does not cover WebSocket messages.
 * These mirror the Rust types in aa-api/src/events.rs, aa-runtime, and
 * aa-gateway. Keep in sync when the Rust types change.
 */

/** Governance audit event enriched with runtime metadata. */
export interface AuditEvent {
  readonly agentId: string;
  readonly action: string;
  readonly decision: "allow" | "deny" | "human_approval";
  readonly reason: string;
  readonly timestamp: number;
}

/** Layer degradation notification. */
export interface LayerDegradationInfo {
  readonly layer: string;
  readonly reason: string;
  readonly remainingLayers: string[];
}

/** Pipeline event — union of audit and degradation events. */
export type PipelineEvent =
  | { type: "audit"; payload: AuditEvent }
  | { type: "layer_degradation"; payload: LayerDegradationInfo };

/** Human-approval request from the approval queue. */
export interface ApprovalRequest {
  readonly requestId: string;
  readonly agentId: string;
  readonly action: string;
  readonly conditionTriggered: string;
  readonly submittedAt: number;
  readonly timeoutSecs: number;
  readonly fallback: "allow" | "deny";
}

/** Budget threshold alert. */
export interface BudgetAlert {
  readonly agentId: string;
  readonly thresholdPct: number;
  readonly spentUsd: number;
  readonly limitUsd: number;
}

/** Union of all WebSocket event messages. */
export type WsEvent =
  | { type: "pipeline"; payload: PipelineEvent }
  | { type: "approval"; payload: ApprovalRequest }
  | { type: "budget"; payload: BudgetAlert };
