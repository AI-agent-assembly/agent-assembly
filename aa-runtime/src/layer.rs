//! Interception layer detection and graceful fallback.
//!
//! The runtime supports three interception layers — eBPF, proxy, and SDK —
//! each detected at startup. [`LayerDetector::detect`] probes system
//! capabilities and returns a [`LayerSet`] bitflag indicating which layers
//! are available.

use std::fmt;

bitflags::bitflags! {
    /// Bitflag set of active interception layers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LayerSet: u8 {
        /// Kernel-level eBPF instrumentation (Linux ≥ 5.8 with BTF and CAP_BPF).
        const EBPF  = 0x1;
        /// Sidecar proxy (`aa-proxy` binary on Linux or macOS).
        const PROXY = 0x2;
        /// In-process SDK hooks (always available).
        const SDK   = 0x4;
    }
}

impl LayerSet {
    /// Return human-readable names for each active layer, in fixed order.
    pub fn names(self) -> Vec<&'static str> {
        let mut out = Vec::with_capacity(3);
        if self.contains(Self::EBPF) {
            out.push("ebpf");
        }
        if self.contains(Self::PROXY) {
            out.push("proxy");
        }
        if self.contains(Self::SDK) {
            out.push("sdk");
        }
        out
    }
}

impl fmt::Display for LayerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = self.names();
        if names.is_empty() {
            return write!(f, "none");
        }
        write!(f, "{}", names.join("+"))
    }
}
