//! TCP accept loop and CONNECT tunnel handling.
//!
//! `ProxyServer` owns the bound TCP listener, the TLS context (CA + cert cache),
//! and the interceptor. It is the top-level runtime object of the proxy.

use std::sync::Arc;
use std::time::SystemTime;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::ServerConfig;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::config::ProxyConfig;
use crate::error::ProxyError;
use crate::intercept::detect::{detect_api, LlmApiPattern};
use crate::intercept::event::ProxyEvent;
use crate::intercept::Interceptor;
use crate::tls::{CaStore, CertCache};

/// The running proxy server.
///
/// Create via [`ProxyServer::new`], then drive the accept loop with
/// [`ProxyServer::run`]. Internally wrapped in [`Arc`] so connection
/// tasks can share the TLS context and interceptor.
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

        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .map_err(|e| ProxyError::Io(e))?;
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .map_err(|e| ProxyError::Io(e))?;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    let (stream, peer) = result?;
                    tracing::debug!(%peer, "accepted connection");
                    let server = Arc::clone(self);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            tracing::warn!(%peer, error = %e, "connection error");
                        }
                    });
                }
                _ = sigint.recv() => {
                    tracing::info!("received SIGINT, shutting down");
                    break;
                }
                _ = sigterm.recv() => {
                    tracing::info!("received SIGTERM, shutting down");
                    break;
                }
            }
        }

        Ok(())
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

            // Extract hostname (strip port) for certificate generation.
            let host = target.split(':').next().unwrap_or(target);

            // When llm_only is enabled, skip TLS MitM for non-LLM hosts and
            // just tunnel the raw TCP bytes transparently.
            if self.config.llm_only && detect_api(host) == LlmApiPattern::Unknown {
                tracing::debug!(%host, "llm_only mode — transparent tunnel (no MitM)");
                let upstream = TcpStream::connect(target).await?;
                let (mut cr, mut cw) = tokio::io::split(stream);
                let (mut ur, mut uw) = tokio::io::split(upstream);
                tokio::select! {
                    r = tokio::io::copy(&mut cr, &mut uw) => { r?; }
                    r = tokio::io::copy(&mut ur, &mut cw) => { r?; }
                }
                return Ok(());
            }

            // --- TLS MitM: act as TLS server to the client ---
            let ck = self.certs.get_or_insert(host, &self.ca)?;
            let cert = CertificateDer::from(ck.cert_der.clone());
            let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(ck.key_der.clone()));
            let server_config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert], key)
                .map_err(|e| ProxyError::Tls(e.to_string()))?;
            let acceptor = TlsAcceptor::from(Arc::new(server_config));
            let client_tls = acceptor
                .accept(stream)
                .await
                .map_err(|e| ProxyError::Tls(e.to_string()))?;

            // --- TLS client: connect to the real upstream ---
            let upstream_tcp = TcpStream::connect(target).await?;
            let mut root_store = rustls::RootCertStore::empty();
            let native = rustls_native_certs::load_native_certs();
            for cert in native.certs {
                let _ = root_store.add(cert);
            }
            let client_config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let connector = TlsConnector::from(Arc::new(client_config));
            let server_name = ServerName::try_from(host.to_string())
                .map_err(|e| ProxyError::Tls(e.to_string()))?;
            let upstream_tls = connector
                .connect(server_name, upstream_tcp)
                .await
                .map_err(|e| ProxyError::Tls(e.to_string()))?;

            tracing::debug!(%host, "TLS MitM handshake complete");

            // Emit interception event for this tunnelled connection.
            let pattern = detect_api(host);
            if pattern != LlmApiPattern::Unknown {
                let event = ProxyEvent {
                    agent_id: None,
                    pattern,
                    method: "CONNECT".into(),
                    path: format!("tunnel to {target}"),
                    request_body: None,
                    response_body: None,
                    timestamp: SystemTime::now(),
                };
                self.interceptor.intercept(event).await?;
            }

            // Bidirectional copy between client and upstream.
            let (mut client_read, mut client_write) = tokio::io::split(client_tls);
            let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream_tls);

            let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
            let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);

            tokio::select! {
                r = client_to_upstream => { r?; }
                r = upstream_to_client => { r?; }
            }

            Ok(())
        } else {
            // Plain HTTP request forwarding.
            tracing::debug!(method = method, target = target, "plain HTTP request");

            // Consume remaining request headers.
            let mut headers = Vec::new();
            let mut header_line = String::new();
            loop {
                header_line.clear();
                reader.read_line(&mut header_line).await?;
                if header_line.trim().is_empty() {
                    break;
                }
                headers.push(header_line.clone());
            }

            // Parse host from the target URL or Host header.
            let host = if let Some(url_host) = target.strip_prefix("http://") {
                url_host.split('/').next().unwrap_or(url_host)
            } else {
                headers
                    .iter()
                    .find_map(|h| {
                        let lower = h.to_ascii_lowercase();
                        lower.starts_with("host:").then(|| h["host:".len()..].trim().to_string())
                    })
                    .unwrap_or_default()
                    .leak()
            };

            // Connect to upstream via plain TCP.
            let upstream_addr = if host.contains(':') {
                host.to_string()
            } else {
                format!("{host}:80")
            };
            let mut upstream = TcpStream::connect(&upstream_addr).await?;

            // Re-serialise and forward the original request.
            upstream.write_all(request_line.as_bytes()).await?;
            upstream.write_all(b"\r\n").await?;
            for h in &headers {
                upstream.write_all(h.as_bytes()).await?;
            }
            upstream.write_all(b"\r\n").await?;

            // Bidirectional copy between client and upstream.
            let stream = reader.into_inner();
            let (mut client_read, mut client_write) = tokio::io::split(stream);
            let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);

            let c2u = tokio::io::copy(&mut client_read, &mut upstream_write);
            let u2c = tokio::io::copy(&mut upstream_read, &mut client_write);

            tokio::select! {
                r = c2u => { r?; }
                r = u2c => { r?; }
            }

            Ok(())
        }
    }
}
