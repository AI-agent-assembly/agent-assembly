//! TCP accept loop and CONNECT tunnel handling.
//!
//! `ProxyServer` owns the bound TCP listener, the TLS context (CA + cert cache),
//! and the interceptor. It is the top-level runtime object of the proxy.

use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::intercept::Interceptor;
use crate::tls::{CaStore, CertCache};

/// The running proxy server.
///
/// Create via [`ProxyServer::new`], then drive the accept loop with
/// [`ProxyServer::run`]. Internally wrapped in [`Arc`] so connection
/// tasks can share the TLS context and interceptor.
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
    pub fn new(config: ProxyConfig, ca: CaStore) -> Arc<Self> {
        let certs = CertCache::new(config.cert_cache_capacity);
        Arc::new(Self {
            config,
            ca,
            certs,
            interceptor: Interceptor::new(),
        })
    }

    /// Bind the TCP listener and enter the accept loop.
    ///
    /// This future runs until the process is killed or an unrecoverable error
    /// occurs. It is called from [`crate::run`].
    pub async fn run(self: &Arc<Self>) -> Result<(), ProxyError> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        tracing::info!(addr = %self.config.bind_addr, "proxy listening");

        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::debug!(%peer, "accepted connection");
            let server = Arc::clone(self);
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream).await {
                    tracing::warn!(%peer, error = %e, "connection error");
                }
            });
        }
    }

    /// Handle a single accepted TCP connection.
    async fn handle_connection(self: &Arc<Self>, _stream: TcpStream) -> Result<(), ProxyError> {
        // Placeholder — CONNECT parsing and forwarding added in subsequent commits.
        Ok(())
    }
}
