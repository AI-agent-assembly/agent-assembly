//! eBPF object loader: parses and loads the compiled eBPF ELF into the kernel.

use aya::Ebpf;

use crate::error::EbpfError;

/// Loads the compiled `aa-ebpf-probes` TLS uprobe ELF object into the Linux kernel.
///
/// The object is embedded at build time by `build.rs`.  `EbpfLoader` is the
/// entry point for all probe attachment in this crate: obtain an [`Ebpf`]
/// handle from [`EbpfLoader::load`] and pass it to the individual managers
/// ([`crate::uprobe::UprobeManager`], [`crate::ringbuf::RingBufReader`], etc.).
pub struct EbpfLoader;

impl EbpfLoader {
    /// Load the embedded TLS uprobe ELF bytecode and return a live [`Ebpf`] handle.
    ///
    /// Parses the `aa-tls-probes` BPF ELF embedded via
    /// [`crate::AA_TLS_BPF`] and submits it to the kernel.  The returned
    /// handle owns the loaded programs; dropping it detaches all probes.
    ///
    /// # Errors
    ///
    /// Returns [`EbpfError::Load`] if the kernel rejects the object (e.g.
    /// missing BTF, kernel too old, or BPF verifier failure).
    ///
    /// # Linux requirements
    ///
    /// Requires Linux 5.8+ with BTF enabled (`CONFIG_DEBUG_INFO_BTF=y`) and
    /// `CAP_BPF` + `CAP_PERFMON` capabilities.
    pub fn load() -> Result<Ebpf, EbpfError> {
        Ok(Ebpf::load(crate::AA_TLS_BPF)?)
    }
}
