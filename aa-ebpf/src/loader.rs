//! eBPF object loader: parses and loads the compiled eBPF ELF into the kernel.

use aya::Bpf;

use crate::error::EbpfError;

/// Loads the compiled `aa-ebpf-programs` ELF object into the Linux kernel.
///
/// The object is embedded at build time by `build.rs` using `aya-build`.
/// `EbpfLoader` is the entry point for all probe attachment in this crate:
/// obtain a [`Bpf`] handle from [`EbpfLoader::load`] and pass it to the
/// individual managers ([`crate::uprobe::UprobeManager`], etc.).
pub struct EbpfLoader;

impl EbpfLoader {
    /// Load the embedded eBPF ELF bytecode and return a live [`Bpf`] handle.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Load`] if the kernel rejects the object (e.g.
    /// missing BTF, kernel too old).
    ///
    /// # Linux requirements
    ///
    /// Requires Linux 5.8+ with BTF enabled (`CONFIG_DEBUG_INFO_BTF=y`).
    pub fn load() -> Result<Bpf, EbpfError> {
        // TODO(AAASM-37): embed eBPF ELF via include_bytes_aligned! from
        // OUT_DIR and call Bpf::load().
        todo!("embed and load aa-ebpf-programs ELF")
    }
}
