//! CA certificate and key management for the MitM proxy.
//!
//! The CA is generated once on first startup and persisted to `~/.aa/ca/`.
//! All subsequent per-domain certificates are signed by this CA.

use std::path::Path;

use crate::error::ProxyError;

/// A signed TLS certificate and its corresponding private key in DER encoding.
///
/// Used as the value stored in [`super::cert::CertCache`].
pub struct CertifiedKey {
    /// DER-encoded certificate chain (leaf cert only for dynamically generated certs).
    pub cert_der: Vec<u8>,
    /// DER-encoded PKCS#8 private key.
    pub key_der: Vec<u8>,
}

/// Holds the local CA certificate and key pair used to sign per-domain certs.
///
/// The CA files on disk are:
/// - `<ca_dir>/ca.crt` — PEM-encoded CA certificate
/// - `<ca_dir>/ca.key` — PEM-encoded CA private key
// Fields are read by sign_cert() once implemented; silence dead_code until then.
#[allow(dead_code)]
pub struct CaStore {
    /// PEM bytes of the CA certificate.
    ca_cert_pem: Vec<u8>,
    /// PEM bytes of the CA private key.
    ca_key_pem: Vec<u8>,
}

impl CaStore {
    /// Load the CA from `ca_dir` if it exists, or generate a new self-signed CA
    /// and persist it before returning.
    pub async fn load_or_create(_ca_dir: &Path) -> Result<Self, ProxyError> {
        todo!()
    }

    /// Generate a DER-encoded leaf certificate for `domain`, signed by this CA.
    pub fn sign_cert(&self, _domain: &str) -> Result<CertifiedKey, ProxyError> {
        todo!()
    }
}
