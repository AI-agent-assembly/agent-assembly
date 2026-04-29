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

// ── eBPF availability probes ──────────────────────────────────────────────────

/// Check whether the running kernel version is ≥ 5.8 (minimum for BPF ring buffer).
///
/// Returns `false` on non-Linux or if the version string cannot be parsed.
fn check_kernel_version() -> bool {
    #[cfg(target_os = "linux")]
    {
        let info = match uname_release() {
            Some(s) => s,
            None => return false,
        };
        parse_kernel_version_ge(&info, 5, 8)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Parse a kernel release string (e.g. `"5.15.0-91-generic"`) and return
/// `true` if major.minor ≥ the given threshold.
fn parse_kernel_version_ge(release: &str, req_major: u32, req_minor: u32) -> bool {
    let mut parts = release.split(|c: char| !c.is_ascii_digit());
    let major = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    (major, minor) >= (req_major, req_minor)
}

/// Read the kernel release string via libc `uname(2)`.
#[cfg(target_os = "linux")]
fn uname_release() -> Option<String> {
    use std::ffi::CStr;
    unsafe {
        let mut info: libc::utsname = std::mem::zeroed();
        if libc::uname(&mut info) != 0 {
            return None;
        }
        CStr::from_ptr(info.release.as_ptr())
            .to_str()
            .ok()
            .map(String::from)
    }
}

/// Check whether BTF type information is available (required by modern eBPF programs).
fn check_btf_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/sys/kernel/btf/vmlinux").exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Simplified CAP_BPF check — returns `true` if running as root (euid 0).
///
/// A full capability check would use `capget(2)` or the `caps` crate, but
/// for the initial implementation root-check is sufficient.
fn check_cap_bpf() -> bool {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: geteuid is always safe to call.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Returns `true` if all eBPF prerequisites are met.
fn probe_ebpf() -> bool {
    check_kernel_version() && check_btf_available() && check_cap_bpf()
}

// ── Proxy availability probe ─────────────────────────────────────────────────

/// Returns `true` if the `aa-proxy` binary is available on a supported platform.
///
/// Supported platforms: Linux and macOS. The binary must be discoverable via `$PATH`.
fn probe_proxy() -> bool {
    let supported_platform = cfg!(target_os = "linux") || cfg!(target_os = "macos");
    supported_platform && which::which("aa-proxy").is_ok()
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
