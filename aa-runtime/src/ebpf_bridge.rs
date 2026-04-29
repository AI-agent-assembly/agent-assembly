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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file_io(syscall: SyscallKind, path: &str) -> FileIoEvent {
        FileIoEvent {
            pid: 100,
            tid: 101,
            timestamp_ns: 5_000_000,
            syscall,
            path: path.to_string(),
            flags: 0,
            return_code: 0,
            is_sensitive: false,
        }
    }

    #[test]
    fn file_io_to_audit_maps_all_syscall_kinds() {
        let cases = [
            (SyscallKind::Openat, "create"),
            (SyscallKind::Read, "read"),
            (SyscallKind::Write, "write"),
            (SyscallKind::Unlink, "delete"),
            (SyscallKind::Rename, "rename"),
        ];
        for (kind, expected_op) in cases {
            let event = make_file_io(kind, "/tmp/test.txt");
            let audit = file_io_to_audit(&event);

            assert_eq!(audit.action_type, ActionType::FileOperation.into());
            let detail = audit.detail.expect("detail should be set");
            match detail {
                Detail::FileOp(ref fop) => {
                    assert_eq!(fop.operation, expected_op, "syscall {kind:?}");
                    assert_eq!(fop.path, "/tmp/test.txt");
                    assert_eq!(fop.source, "ebpf");
                }
                _ => panic!("expected FileOp detail, got {detail:?}"),
            }
        }
    }
}
