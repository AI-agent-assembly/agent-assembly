//! Error types for the `aa-proxy` crate.

use thiserror::Error;

/// All errors that can arise within `aa-proxy`.
#[derive(Debug, Error)]
pub enum ProxyError {
    /// An underlying I/O error (bind failure, connection reset, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A TLS handshake or configuration error.
    #[error("TLS error: {0}")]
    Tls(String),

    /// A certificate generation error (rcgen failure).
    #[error("Certificate generation error: {0}")]
    CertGen(String),

    /// A configuration error (missing or invalid env var).
    #[error("Configuration error: {0}")]
    Config(String),

    /// A macOS Keychain operation failed (security CLI returned non-zero).
    #[error("Keychain error: {0}")]
    Keychain(String),
}
