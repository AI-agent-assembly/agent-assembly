//! Runtime configuration for `aa-proxy`.

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::error::ProxyError;

/// Runtime configuration for the proxy sidecar.
///
/// All fields can be overridden via environment variables.
pub struct ProxyConfig {
    /// TCP address the proxy listens on.
    /// Env: `AA_PROXY_ADDR` — default: `127.0.0.1:8899`
    pub bind_addr: SocketAddr,

    /// Directory where the CA certificate and key are stored.
    /// Env: `AA_CA_DIR` — default: `~/.aa/ca/`
    pub ca_dir: PathBuf,

    /// Maximum number of dynamically generated certificates to cache.
    /// Default: 1000
    pub cert_cache_capacity: usize,

    /// When `true`, only LLM API traffic is intercepted; all other HTTPS is
    /// forwarded transparently.
    /// Env: `AA_PROXY_LLM_ONLY` — default: `true`
    pub llm_only: bool,
}

impl ProxyConfig {
    /// Build a `ProxyConfig` from environment variables, falling back to
    /// defaults where variables are not set.
    pub fn from_env() -> Result<Self, ProxyError> {
        todo!()
    }
}
