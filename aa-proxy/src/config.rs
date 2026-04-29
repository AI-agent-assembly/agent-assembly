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
        let bind_addr = match std::env::var("AA_PROXY_ADDR") {
            Ok(val) => val
                .parse::<SocketAddr>()
                .map_err(|e| ProxyError::Config(format!("invalid AA_PROXY_ADDR: {e}")))?,
            Err(_) => SocketAddr::from(([127, 0, 0, 1], 8899)),
        };

        let ca_dir = match std::env::var("AA_CA_DIR") {
            Ok(val) => PathBuf::from(val),
            Err(_) => dirs::home_dir()
                .ok_or_else(|| ProxyError::Config("cannot determine home directory".into()))?
                .join(".aa")
                .join("ca"),
        };

        let cert_cache_capacity = match std::env::var("AA_PROXY_CERT_CACHE_CAPACITY") {
            Ok(val) => val
                .parse::<usize>()
                .map_err(|e| ProxyError::Config(format!("invalid AA_PROXY_CERT_CACHE_CAPACITY: {e}")))?,
            Err(_) => 1000,
        };

        let llm_only = match std::env::var("AA_PROXY_LLM_ONLY") {
            Ok(val) => val != "0" && val.to_lowercase() != "false",
            Err(_) => true,
        };

        Ok(Self {
            bind_addr,
            ca_dir,
            cert_cache_capacity,
            llm_only,
        })
    }
}
