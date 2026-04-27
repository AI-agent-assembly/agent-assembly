//! IPC message types for the Unix domain socket protocol.
//!
//! `IpcFrame` represents messages arriving from an SDK process (inbound).
//! `IpcResponse` represents messages sent back to an SDK process (outbound).

use aa_proto::assembly::audit::v1::AuditEvent;
use aa_proto::assembly::event::v1::ApprovalDecision;
use aa_proto::assembly::policy::v1::CheckActionRequest;

/// A decoded message received from an SDK process over the Unix socket.
///
/// Each variant corresponds to a 1-byte wire tag:
/// - `1` = PolicyQuery
/// - `2` = EventReport
/// - `3` = ApprovalResponse
/// - `4` = Heartbeat
#[derive(Debug)]
pub enum IpcFrame {
    /// A policy check request — SDK asks the runtime to evaluate an action.
    PolicyQuery(CheckActionRequest),
    /// An audit event report — SDK sends a governance event for recording.
    EventReport(AuditEvent),
    /// An approval decision — SDK sends the human reviewer's verdict.
    ApprovalResponse(ApprovalDecision),
    /// A liveness ping — no payload. Runtime echoes an `Ack`.
    Heartbeat,
}
