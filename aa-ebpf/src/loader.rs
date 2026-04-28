//! eBPF program loader for the userspace side of Agent Assembly.
//!
//! Responsible for loading compiled BPF bytecode via [`aya::Ebpf`],
//! attaching tracepoints and kprobes, and setting up ring buffer
//! consumers that feed events into the governance pipeline.

/// Loads and manages the lifecycle of eBPF programs.
///
/// On construction, loads the compiled BPF bytecode and attaches the
/// configured probes. On drop, detaches all probes and releases
/// kernel resources.
pub struct EbpfLoader {
    _private: (),
}
