//! Kprobe modules for file I/O syscall interception.
//!
//! Each submodule implements the attachment and event handling logic for
//! one syscall kprobe. The actual eBPF program bytecode is compiled
//! separately (see `aa-ebpf-probes`) and loaded by [`crate::loader::EbpfLoader`].

pub mod openat;
pub mod read;
pub mod rename;
pub mod unlink;
pub mod write;
