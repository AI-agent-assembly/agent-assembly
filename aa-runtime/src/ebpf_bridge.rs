//! Bridge between eBPF kernel events and the runtime pipeline.
//!
//! Maps raw eBPF event types from `aa_ebpf` into `AuditEvent` proto messages
//! and enriches them for the broadcast channel.
