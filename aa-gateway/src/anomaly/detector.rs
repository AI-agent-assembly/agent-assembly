//! Anomaly detection logic.
//!
//! Compares current agent activity against the per-agent behavioral baseline
//! (see [`super::baseline`]) to identify each of the seven anomaly types.
//!
//! Detection approaches per type:
//!
//! | # | Type                          | Approach                                                     |
//! |---|-------------------------------|--------------------------------------------------------------|
//! | 1 | BehaviorSpike                 | Current-window action rate vs baseline mean + N*stddev       |
//! | 2 | UnknownExternalConnection     | Cross-reference host against `network.allowlist`             |
//! | 3 | CredentialLeakAttempt          | Count `CredentialFinding` events per window; alert if > threshold |
//! | 4 | ChildProcessExecution         | Any `ProcessExec` action not in tool allowlist               |
//! | 5 | DataExfiltrationAttempt        | PII findings + `NetworkRequest` to external host in same trace |
//! | 6 | LoopRunaway                   | Count identical `(tool_name, args_hash)` pairs in window     |
//! | 7 | CrossAgentIdentitySpoofing    | Agent A's credential token used with Agent B's agent_id      |

// TODO(AAASM-137): Implement AnomalyDetector struct with:
//   - detect(&self, agent_id, action, baseline) -> Option<AnomalyEvent>
//   - per-type detection methods
//   - configurable thresholds (stddev multiplier, loop count, window duration)
