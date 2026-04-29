//! TCP accept loop and CONNECT tunnel handling.
//!
//! `ProxyServer` owns the bound TCP listener, the TLS context (CA + cert cache),
//! and the interceptor. It is the top-level runtime object of the proxy.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    ///
    /// Reads the first HTTP request line to determine whether this is a
    /// `CONNECT` tunnel (HTTPS) or a plain HTTP request.
    async fn handle_connection(self: &Arc<Self>, stream: TcpStream) -> Result<(), ProxyError> {
        let mut reader = BufReader::new(stream);

        // Read the first request line, e.g. "CONNECT api.openai.com:443 HTTP/1.1\r\n"
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;
        let request_line = request_line.trim_end();

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(ProxyError::Config("malformed HTTP request line".into()));
        }

        let method = parts[0];
        let target = parts[1];

        if method.eq_ignore_ascii_case("CONNECT") {
            // Consume remaining headers (we only need the request line for CONNECT).
            let mut header_line = String::new();
            loop {
                header_line.clear();
                reader.read_line(&mut header_line).await?;
                if header_line.trim().is_empty() {
                    break;
                }
            }

            // Send 200 Connection Established to tell the client the tunnel is open.
            let inner = reader.into_inner();
            let mut stream = inner;
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await?;

            tracing::debug!(host = target, "CONNECT tunnel established");
            // TLS MitM and forwarding added in subsequent commits.
            Ok(())
        } else {
            // Plain HTTP — forwarding added in a subsequent commit.
            tracing::debug!(method, target, "plain HTTP request");
            Ok(())
        }
    }
}
