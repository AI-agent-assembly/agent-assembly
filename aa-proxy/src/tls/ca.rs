//! CA certificate and key management for the MitM proxy.
//!
//! The CA is generated once on first startup and persisted to `~/.aa/ca/`.
//! All subsequent per-domain certificates are signed by this CA.

use std::path::{Path, PathBuf};

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};
use rcgen::PKCS_ECDSA_P256_SHA256;
use time::{Duration, OffsetDateTime};

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
/// - `<ca_dir>/ca-cert.pem` — PEM-encoded CA certificate
/// - `<ca_dir>/ca-key.pem`  — PEM-encoded CA private key (chmod 600)
pub struct CaStore {
    /// Directory where CA files are persisted.
    pub(crate) ca_dir: PathBuf,
    /// PEM-encoded CA certificate (used for signing and keychain install).
    pub(crate) ca_cert_pem: String,
    /// PEM-encoded CA private key (used for signing leaf certs).
    pub(crate) ca_key_pem: String,
}

impl CaStore {
    /// Load the CA from `ca_dir` if it exists, or generate a new self-signed CA
    /// and persist it before returning.
    pub async fn load_or_create(ca_dir: &Path) -> Result<Self, ProxyError> {
        let cert_path = ca_dir.join("ca-cert.pem");
        let key_path  = ca_dir.join("ca-key.pem");

        // Load existing CA if both files are present.
        if cert_path.exists() && key_path.exists() {
            let ca_cert_pem = tokio::fs::read_to_string(&cert_path).await?;
            let ca_key_pem  = tokio::fs::read_to_string(&key_path).await?;
            return Ok(Self { ca_dir: ca_dir.to_path_buf(), ca_cert_pem, ca_key_pem });
        }

        // Generate a new EC P-256 CA key pair.
        let ca_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| ProxyError::CertGen(e.to_string()))?;

        let mut ca_params = CertificateParams::new(vec![])
            .map_err(|e| ProxyError::CertGen(e.to_string()))?;
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        ca_params.distinguished_name.push(DnType::CommonName, "Agent Assembly CA");
        ca_params.not_before = OffsetDateTime::now_utc();
        ca_params.not_after  = OffsetDateTime::now_utc()
            .checked_add(Duration::days(365 * 10))
            .expect("date arithmetic cannot overflow for 10-year span");

        let ca_cert     = ca_params.self_signed(&ca_key)
            .map_err(|e| ProxyError::CertGen(e.to_string()))?;
        let ca_cert_pem = ca_cert.pem();
        let ca_key_pem  = ca_key.serialize_pem();

        // Persist to disk.
        tokio::fs::create_dir_all(ca_dir).await?;
        tokio::fs::write(&cert_path, &ca_cert_pem).await?;
        tokio::fs::write(&key_path,  &ca_key_pem).await?;

        // Restrict key file to owner read/write only.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).await?;
        }

        Ok(Self { ca_dir: ca_dir.to_path_buf(), ca_cert_pem, ca_key_pem })
    }

    /// Generate a DER-encoded leaf certificate for `domain`, signed by this CA.
    pub fn sign_cert(&self, _domain: &str) -> Result<CertifiedKey, ProxyError> {
        todo!()
    }

    /// Install the CA certificate into the macOS System Keychain as a trusted root.
    /// No-op if already installed.
    #[cfg(target_os = "macos")]
    pub fn install(&self) -> Result<(), ProxyError> {
        todo!()
    }

    /// Return `true` if this CA is currently trusted by the macOS System Keychain.
    #[cfg(target_os = "macos")]
    pub fn is_installed(&self) -> Result<bool, ProxyError> {
        todo!()
    }

    /// Remove this CA from the macOS System Keychain and delete `ca_dir` from disk.
    #[cfg(target_os = "macos")]
    pub fn uninstall(&self) -> Result<(), ProxyError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn load_or_create_generates_pem_files() {
        let dir = TempDir::new().unwrap();
        CaStore::load_or_create(dir.path()).await.unwrap();
        assert!(dir.path().join("ca-cert.pem").exists(), "ca-cert.pem missing");
        assert!(dir.path().join("ca-key.pem").exists(), "ca-key.pem missing");
    }

    #[tokio::test]
    async fn load_or_create_returns_valid_pem() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        assert!(ca.ca_cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(ca.ca_key_pem.contains("-----BEGIN PRIVATE KEY-----"));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn load_or_create_key_file_is_chmod_600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        CaStore::load_or_create(dir.path()).await.unwrap();
        let perms = std::fs::metadata(dir.path().join("ca-key.pem"))
            .unwrap()
            .permissions();
        assert_eq!(perms.mode() & 0o777, 0o600, "ca-key.pem must be owner-read-write only");
    }

    #[tokio::test]
    async fn load_or_create_reload_returns_same_cert() {
        let dir = TempDir::new().unwrap();
        let ca1 = CaStore::load_or_create(dir.path()).await.unwrap();
        let ca2 = CaStore::load_or_create(dir.path()).await.unwrap();
        assert_eq!(ca1.ca_cert_pem, ca2.ca_cert_pem, "reload must return identical cert");
    }
}
