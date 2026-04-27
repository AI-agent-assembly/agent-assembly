//! TCP accept loop and CONNECT tunnel handling.
//!
//! `ProxyServer` owns the bound TCP listener, the TLS context (CA + cert cache),
//! and the interceptor. It is the top-level runtime object of the proxy.

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::intercept::Interceptor;
use crate::tls::{CaStore, CertCache};

/// The running proxy server.
///
/// Create via [`ProxyServer::new`], then drive the accept loop with
/// [`ProxyServer::run`].
// Fields are read by `run()` once implemented; silence dead_code until then.
#[allow(dead_code)]
pub struct ProxyServer {
    config: ProxyConfig,
    ca: CaStore,
    certs: CertCache,
    interceptor: Interceptor,
}

impl ProxyServer {
    /// Construct a `ProxyServer` from a validated config and an initialised CA.
    pub fn new(config: ProxyConfig, ca: CaStore) -> Self {
        let certs = CertCache::new(config.cert_cache_capacity);
        Self {
            config,
            ca,
            certs,
            interceptor: Interceptor::new(),
        }
    }

    /// Bind the TCP listener and enter the accept loop.
    ///
    /// This future runs until the process is killed or an unrecoverable error
    /// occurs. It is called from [`crate::run`].
    pub async fn run(&self) -> Result<(), ProxyError> {
        todo!()
    }
}
