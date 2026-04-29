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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn individual_flags_have_correct_bits() {
        assert_eq!(LayerSet::EBPF.bits(), 0x1);
        assert_eq!(LayerSet::PROXY.bits(), 0x2);
        assert_eq!(LayerSet::SDK.bits(), 0x4);
    }

    #[test]
    fn flags_combine_with_bitor() {
        let set = LayerSet::EBPF | LayerSet::SDK;
        assert!(set.contains(LayerSet::EBPF));
        assert!(set.contains(LayerSet::SDK));
        assert!(!set.contains(LayerSet::PROXY));
    }

    #[test]
    fn names_returns_active_layers_in_order() {
        let all = LayerSet::EBPF | LayerSet::PROXY | LayerSet::SDK;
        assert_eq!(all.names(), vec!["ebpf", "proxy", "sdk"]);

        let sdk_only = LayerSet::SDK;
        assert_eq!(sdk_only.names(), vec!["sdk"]);

        let proxy_sdk = LayerSet::PROXY | LayerSet::SDK;
        assert_eq!(proxy_sdk.names(), vec!["proxy", "sdk"]);
    }

    #[test]
    fn names_empty_for_empty_set() {
        let empty = LayerSet::empty();
        assert!(empty.names().is_empty());
    }

    #[test]
    fn display_joins_with_plus() {
        let all = LayerSet::EBPF | LayerSet::PROXY | LayerSet::SDK;
        assert_eq!(format!("{all}"), "ebpf+proxy+sdk");
    }

    #[test]
    fn display_sdk_only() {
        assert_eq!(format!("{}", LayerSet::SDK), "sdk");
    }

    #[test]
    fn display_empty_shows_none() {
        assert_eq!(format!("{}", LayerSet::empty()), "none");
    }
}
