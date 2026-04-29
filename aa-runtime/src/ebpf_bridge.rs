//! Bridge between eBPF kernel events and the runtime pipeline.
//!
//! Maps raw eBPF event types from `aa_ebpf` into `AuditEvent` proto messages
//! and enriches them for the broadcast channel.

use aa_ebpf::events::FileIoEvent;
use aa_ebpf::syscall::SyscallKind;
use aa_proto::assembly::audit::v1::audit_event::Detail;
use aa_proto::assembly::audit::v1::{AuditEvent, FileOpDetail};
use aa_proto::assembly::common::v1::ActionType;

/// Convert a file I/O eBPF event into an [`AuditEvent`] proto message.
///
/// Maps `SyscallKind` to the proto `operation` string and populates
/// a `FileOpDetail` with the path and detection source set to `"ebpf"`.
pub fn file_io_to_audit(event: &FileIoEvent) -> AuditEvent {
    let operation = match event.syscall {
        SyscallKind::Openat => "create",
        SyscallKind::Read => "read",
        SyscallKind::Write => "write",
        SyscallKind::Unlink => "delete",
        SyscallKind::Rename => "rename",
    }
    .to_string();

    AuditEvent {
        action_type: ActionType::FileOperation.into(),
        detail: Some(Detail::FileOp(FileOpDetail {
            operation,
            path: event.path.clone(),
            bytes: 0,
            source: "ebpf".to_string(),
        })),
        ..AuditEvent::default()
    }
}
