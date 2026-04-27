# AAASM-42: CA Certificate Provisioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement EC P-256 CA key generation, disk persistence, per-domain leaf cert signing, LRU cert caching, and macOS System Keychain trust management in `aa-proxy`.

**Architecture:** `ca.rs` is platform-agnostic (key generation, file I/O, signing). New `keychain.rs` is macOS-only (`#[cfg(target_os = "macos")]`) and owns all `security` CLI calls. `cert.rs` implements the LRU cache wrapper. All three are wired together in `lib.rs::run()`.

**Tech Stack:** `rcgen 0.13` (EC P-256 key/cert generation), `time 0.3` (cert validity dates), `lru 0.16` (cert cache), `tokio::fs` (async file I/O), `std::process::Command` (macOS `security` CLI), `tempfile 3` (test temp dirs).

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `aa-proxy/Cargo.toml` | Modify | Add `tempfile` dev dep |
| `aa-proxy/src/error.rs` | Modify | Add `Keychain(String)` variant |
| `aa-proxy/src/tls/keychain.rs` | Create | macOS `security` CLI wrappers |
| `aa-proxy/src/tls/mod.rs` | Modify | Add `mod keychain` |
| `aa-proxy/src/tls/ca.rs` | Modify | Implement all `CaStore` methods |
| `aa-proxy/src/tls/cert.rs` | Modify | Implement `CertCache::get_or_insert` |
| `aa-proxy/src/lib.rs` | Modify | Wire `is_installed` + `install` into `run()` |

---

### Task 1: Add dev dependency and Keychain error variant

**Files:**
- Modify: `aa-proxy/Cargo.toml`
- Modify: `aa-proxy/src/error.rs`

- [ ] **Step 1: Add `tempfile` to dev-dependencies**

Open `aa-proxy/Cargo.toml` and add after `[lints]`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Add `Keychain` variant to `ProxyError`**

Replace the full contents of `aa-proxy/src/error.rs`:

```rust
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
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check -p aa-proxy
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add aa-proxy/Cargo.toml aa-proxy/src/error.rs
git commit -m "✨ (error): Add Keychain variant to ProxyError"
```

---

### Task 2: Scaffold `keychain.rs` with function stubs

**Files:**
- Create: `aa-proxy/src/tls/keychain.rs`
- Modify: `aa-proxy/src/tls/mod.rs`

- [ ] **Step 1: Create `aa-proxy/src/tls/keychain.rs`**

```rust
//! macOS System Keychain operations for CA certificate trust management.
//!
//! All functions invoke the `security` CLI via `std::process::Command`.
//! This entire module is macOS-only.

use std::path::Path;

use crate::error::ProxyError;

/// Install the certificate at `cert_path` into the macOS System Keychain
/// as a trusted root CA.
///
/// Invokes: `security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain <cert>`
///
/// Requires admin privileges — macOS prompts for authentication automatically.
#[cfg(target_os = "macos")]
pub(super) fn add_trusted_cert(_cert_path: &Path) -> Result<(), ProxyError> {
    todo!()
}

/// Remove the certificate at `cert_path` from the macOS System Keychain trust.
///
/// Invokes: `security remove-trusted-cert -d <cert>`
///
/// Requires admin privileges — macOS prompts for authentication automatically.
#[cfg(target_os = "macos")]
pub(super) fn remove_trusted_cert(_cert_path: &Path) -> Result<(), ProxyError> {
    todo!()
}

/// Return `true` if a certificate with common name `subject` exists in the
/// macOS System Keychain.
///
/// Invokes: `security find-certificate -c <subject> -a /Library/Keychains/System.keychain`
#[cfg(target_os = "macos")]
pub(super) fn is_cert_trusted(_subject: &str) -> Result<bool, ProxyError> {
    todo!()
}
```

- [ ] **Step 2: Add `mod keychain` to `aa-proxy/src/tls/mod.rs`**

Replace the full contents of `aa-proxy/src/tls/mod.rs`:

```rust
//! TLS subsystem: CA management and per-domain certificate caching.

pub mod ca;
pub mod cert;
mod keychain;

pub use ca::{CaStore, CertifiedKey};
pub use cert::CertCache;
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check -p aa-proxy
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add aa-proxy/src/tls/keychain.rs aa-proxy/src/tls/mod.rs
git commit -m "✨ (tls): Scaffold keychain.rs with security CLI stubs"
```

---

### Task 3: Implement and test `CaStore::load_or_create`

**Files:**
- Modify: `aa-proxy/src/tls/ca.rs`

- [ ] **Step 1: Write the failing tests**

Replace the full contents of `aa-proxy/src/tls/ca.rs`:

```rust
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
    pub async fn load_or_create(_ca_dir: &Path) -> Result<Self, ProxyError> {
        todo!()
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
```

- [ ] **Step 2: Run tests — verify they fail on `todo!()`**

```bash
cargo test -p aa-proxy load_or_create 2>&1 | tail -20
```

Expected: tests fail with `not yet implemented` panic.

- [ ] **Step 3: Implement `load_or_create`**

Replace only the `load_or_create` method body in `aa-proxy/src/tls/ca.rs`:

```rust
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
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cargo test -p aa-proxy load_or_create 2>&1 | tail -20
```

Expected: all `load_or_create_*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add aa-proxy/src/tls/ca.rs
git commit -m "✨ (tls/ca): Implement load_or_create — generate and persist EC P-256 CA"
```

---

### Task 4: Implement and test `CaStore::sign_cert`

**Files:**
- Modify: `aa-proxy/src/tls/ca.rs`

- [ ] **Step 1: Write the failing tests**

Add the following tests to the `#[cfg(test)] mod tests` block at the bottom of `aa-proxy/src/tls/ca.rs` (inside the existing `tests` module, after the `load_or_create_reload_returns_same_cert` test):

```rust
    #[tokio::test]
    async fn sign_cert_returns_non_empty_der() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        let ck = ca.sign_cert("api.openai.com").unwrap();
        assert!(!ck.cert_der.is_empty(), "cert DER must not be empty");
        assert!(!ck.key_der.is_empty(), "key DER must not be empty");
    }

    #[tokio::test]
    async fn sign_cert_different_domains_produce_different_certs() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        let ck1 = ca.sign_cert("api.openai.com").unwrap();
        let ck2 = ca.sign_cert("api.anthropic.com").unwrap();
        assert_ne!(ck1.cert_der, ck2.cert_der, "different domains must produce different certs");
    }

    #[tokio::test]
    async fn sign_cert_same_domain_produces_fresh_cert_each_call() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        let ck1 = ca.sign_cert("api.openai.com").unwrap();
        let ck2 = ca.sign_cert("api.openai.com").unwrap();
        // sign_cert generates a fresh key each call; keys must differ
        assert_ne!(ck1.key_der, ck2.key_der, "each call generates a fresh key pair");
    }
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test -p aa-proxy sign_cert 2>&1 | tail -10
```

Expected: fail with `not yet implemented`.

- [ ] **Step 3: Implement `sign_cert`**

Replace only the `sign_cert` method body in `aa-proxy/src/tls/ca.rs`:

```rust
pub fn sign_cert(&self, domain: &str) -> Result<CertifiedKey, ProxyError> {
    // Reconstruct the CA key and cert from stored PEM.
    let ca_key = KeyPair::from_pem(&self.ca_key_pem)
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;
    let ca_params = CertificateParams::from_ca_cert_pem(&self.ca_cert_pem)
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;
    let ca_cert = ca_params.self_signed(&ca_key)
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;

    // Generate a fresh EC P-256 leaf key and cert for `domain`.
    let leaf_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;
    let mut leaf_params = CertificateParams::new(vec![domain.to_string()])
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;
    leaf_params.not_before = OffsetDateTime::now_utc();
    leaf_params.not_after  = OffsetDateTime::now_utc()
        .checked_add(Duration::days(365))
        .expect("date arithmetic cannot overflow for 1-year span");

    let leaf_cert = leaf_params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .map_err(|e| ProxyError::CertGen(e.to_string()))?;

    Ok(CertifiedKey {
        cert_der: leaf_cert.der().to_vec(),
        key_der:  leaf_key.serialize_der(),
    })
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cargo test -p aa-proxy sign_cert 2>&1 | tail -10
```

Expected: all three `sign_cert_*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add aa-proxy/src/tls/ca.rs
git commit -m "✨ (tls/ca): Implement sign_cert — per-domain leaf cert signed by CA"
```

---

### Task 5: Implement and test `CertCache::get_or_insert`

**Files:**
- Modify: `aa-proxy/src/tls/cert.rs`

- [ ] **Step 1: Write the failing tests**

Replace the full contents of `aa-proxy/src/tls/cert.rs`:

```rust
//! LRU cache for dynamically generated per-domain TLS certificates.
//!
//! Generating a certificate with rcgen takes ~0.1 ms. This cache avoids
//! regenerating a cert for every connection to the same domain.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use lru::LruCache;

use crate::error::ProxyError;
use crate::tls::ca::{CaStore, CertifiedKey};

/// Thread-safe LRU cache mapping domain names to their signed [`CertifiedKey`].
pub struct CertCache {
    inner: Mutex<LruCache<String, Arc<CertifiedKey>>>,
}

impl CertCache {
    /// Create a new cache with the given `capacity` (maximum number of entries).
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("cert cache capacity must be non-zero"),
            )),
        }
    }

    /// Return the cached [`CertifiedKey`] for `domain`, generating and inserting
    /// a new one (via `ca.sign_cert()`) if the domain is not in the cache.
    pub fn get_or_insert(&self, _domain: &str, _ca: &CaStore) -> Result<Arc<CertifiedKey>, ProxyError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn get_or_insert_returns_cert_on_cache_miss() {
        let dir = TempDir::new().unwrap();
        let ca    = CaStore::load_or_create(dir.path()).await.unwrap();
        let cache = CertCache::new(10);
        let ck = cache.get_or_insert("api.openai.com", &ca).unwrap();
        assert!(!ck.cert_der.is_empty());
    }

    #[tokio::test]
    async fn get_or_insert_returns_same_arc_on_cache_hit() {
        let dir = TempDir::new().unwrap();
        let ca    = CaStore::load_or_create(dir.path()).await.unwrap();
        let cache = CertCache::new(10);
        let ck1 = cache.get_or_insert("api.openai.com", &ca).unwrap();
        let ck2 = cache.get_or_insert("api.openai.com", &ca).unwrap();
        // Identical Arc pointer proves cache hit — no re-signing occurred.
        assert!(Arc::ptr_eq(&ck1, &ck2), "second call must return the cached Arc");
    }

    #[tokio::test]
    async fn get_or_insert_different_domains_get_different_certs() {
        let dir = TempDir::new().unwrap();
        let ca    = CaStore::load_or_create(dir.path()).await.unwrap();
        let cache = CertCache::new(10);
        let ck1 = cache.get_or_insert("api.openai.com", &ca).unwrap();
        let ck2 = cache.get_or_insert("api.anthropic.com", &ca).unwrap();
        assert!(!Arc::ptr_eq(&ck1, &ck2));
        assert_ne!(ck1.cert_der, ck2.cert_der);
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test -p aa-proxy get_or_insert 2>&1 | tail -10
```

Expected: fail with `not yet implemented`.

- [ ] **Step 3: Implement `get_or_insert`**

Replace only the `get_or_insert` method body:

```rust
pub fn get_or_insert(&self, domain: &str, ca: &CaStore) -> Result<Arc<CertifiedKey>, ProxyError> {
    let mut cache = self.inner.lock().expect("cert cache lock poisoned");
    if let Some(ck) = cache.get(domain) {
        return Ok(Arc::clone(ck));
    }
    let ck = Arc::new(ca.sign_cert(domain)?);
    cache.put(domain.to_string(), Arc::clone(&ck));
    Ok(ck)
}
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cargo test -p aa-proxy get_or_insert 2>&1 | tail -10
```

Expected: all three `get_or_insert_*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add aa-proxy/src/tls/cert.rs
git commit -m "✨ (tls/cert): Implement get_or_insert — LRU cache with sign_cert on miss"
```

---

### Task 6: Implement `keychain.rs` functions

**Files:**
- Modify: `aa-proxy/src/tls/keychain.rs`

- [ ] **Step 1: Implement all three functions**

Replace the full contents of `aa-proxy/src/tls/keychain.rs`:

```rust
//! macOS System Keychain operations for CA certificate trust management.
//!
//! All functions invoke the `security` CLI via `std::process::Command`.
//! This entire module is macOS-only.

use std::path::Path;
use std::process::Command;

use crate::error::ProxyError;

/// Install the certificate at `cert_path` into the macOS System Keychain
/// as a trusted root CA.
///
/// Invokes: `security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain <cert>`
///
/// Requires admin privileges — macOS prompts for authentication automatically.
#[cfg(target_os = "macos")]
pub(super) fn add_trusted_cert(cert_path: &Path) -> Result<(), ProxyError> {
    let cert_str = cert_path
        .to_str()
        .ok_or_else(|| ProxyError::Keychain("cert path is not valid UTF-8".into()))?;

    let output = Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",
            "-r", "trustRoot",
            "-k", "/Library/Keychains/System.keychain",
            cert_str,
        ])
        .output()
        .map_err(|e| ProxyError::Keychain(format!("failed to run security CLI: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(ProxyError::Keychain(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

/// Remove the certificate at `cert_path` from the macOS System Keychain trust.
///
/// Invokes: `security remove-trusted-cert -d <cert>`
///
/// Requires admin privileges — macOS prompts for authentication automatically.
#[cfg(target_os = "macos")]
pub(super) fn remove_trusted_cert(cert_path: &Path) -> Result<(), ProxyError> {
    let cert_str = cert_path
        .to_str()
        .ok_or_else(|| ProxyError::Keychain("cert path is not valid UTF-8".into()))?;

    let output = Command::new("security")
        .args(["remove-trusted-cert", "-d", cert_str])
        .output()
        .map_err(|e| ProxyError::Keychain(format!("failed to run security CLI: {e}")))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(ProxyError::Keychain(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

/// Return `true` if a certificate with common name `subject` exists in the
/// macOS System Keychain.
///
/// Invokes: `security find-certificate -c <subject> -a /Library/Keychains/System.keychain`
#[cfg(target_os = "macos")]
pub(super) fn is_cert_trusted(subject: &str) -> Result<bool, ProxyError> {
    let output = Command::new("security")
        .args([
            "find-certificate",
            "-c", subject,
            "-a",
            "/Library/Keychains/System.keychain",
        ])
        .output()
        .map_err(|e| ProxyError::Keychain(format!("failed to run security CLI: {e}")))?;

    Ok(output.status.success())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check -p aa-proxy
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add aa-proxy/src/tls/keychain.rs
git commit -m "✨ (tls/keychain): Implement security CLI operations"
```

---

### Task 7: Implement and test `CaStore::install`, `is_installed`, `uninstall`

**Files:**
- Modify: `aa-proxy/src/tls/ca.rs`

- [ ] **Step 1: Implement the three methods**

Replace the three `todo!()` method bodies in `aa-proxy/src/tls/ca.rs`:

```rust
#[cfg(target_os = "macos")]
pub fn install(&self) -> Result<(), ProxyError> {
    if self.is_installed()? {
        return Ok(()); // Already trusted — no-op.
    }
    super::keychain::add_trusted_cert(&self.ca_dir.join("ca-cert.pem"))
}

#[cfg(target_os = "macos")]
pub fn is_installed(&self) -> Result<bool, ProxyError> {
    super::keychain::is_cert_trusted("Agent Assembly CA")
}

#[cfg(target_os = "macos")]
pub fn uninstall(&self) -> Result<(), ProxyError> {
    super::keychain::remove_trusted_cert(&self.ca_dir.join("ca-cert.pem"))?;
    std::fs::remove_dir_all(&self.ca_dir)?;
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check -p aa-proxy
```

Expected: no errors.

- [ ] **Step 3: Add keychain integration tests (macOS only, `#[ignore]`)**

Add the following module at the very bottom of `aa-proxy/src/tls/ca.rs`, after the existing `tests` module:

```rust
/// Integration tests for macOS Keychain operations.
///
/// These tests require:
/// - macOS (System Keychain)
/// - Admin privileges (macOS will prompt via GUI)
///
/// Run with: `cargo test -p aa-proxy -- --ignored keychain`
#[cfg(all(test, target_os = "macos"))]
mod keychain_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    #[ignore = "requires macOS System Keychain write access (admin auth prompt)"]
    async fn install_makes_ca_trusted() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        ca.install().unwrap();
        assert!(ca.is_installed().unwrap(), "CA must be trusted after install");
        // Cleanup: remove from keychain so test is idempotent.
        super::keychain::remove_trusted_cert(&dir.path().join("ca-cert.pem")).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires macOS System Keychain write access (admin auth prompt)"]
    async fn uninstall_removes_ca_and_deletes_dir() {
        let dir = TempDir::new().unwrap();
        let dir_path = dir.path().to_path_buf();
        let ca = CaStore::load_or_create(&dir_path).await.unwrap();
        ca.install().unwrap();
        assert!(ca.is_installed().unwrap());

        ca.uninstall().unwrap();
        assert!(!ca.is_installed().unwrap(), "CA must not be trusted after uninstall");
        assert!(!dir_path.exists(), "ca_dir must be deleted after uninstall");
        // TempDir will try to clean up, but the dir is already gone — that's fine.
        std::mem::forget(dir);
    }

    #[tokio::test]
    #[ignore = "requires macOS System Keychain write access (admin auth prompt)"]
    async fn install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let ca = CaStore::load_or_create(dir.path()).await.unwrap();
        ca.install().unwrap();
        ca.install().unwrap(); // Second call must not fail.
        assert!(ca.is_installed().unwrap());
        // Cleanup.
        super::keychain::remove_trusted_cert(&dir.path().join("ca-cert.pem")).unwrap();
    }
}
```

- [ ] **Step 4: Run unit tests (non-ignored) to confirm nothing regressed**

```bash
cargo test -p aa-proxy 2>&1 | tail -20
```

Expected: all non-`#[ignore]` tests pass, ignored tests listed but skipped.

- [ ] **Step 5: Commit**

```bash
git add aa-proxy/src/tls/ca.rs
git commit -m "✨ (tls/ca): Add install, is_installed, uninstall to CaStore"
```

---

### Task 8: Wire `is_installed` + `install` into `run()` and run full suite

**Files:**
- Modify: `aa-proxy/src/lib.rs`

- [ ] **Step 1: Update `run()` to install CA trust if needed**

Replace the `run` function in `aa-proxy/src/lib.rs`:

```rust
/// Start the proxy with the given configuration.
///
/// Loads or creates the CA from `config.ca_dir`, installs it into the macOS
/// System Keychain if not already trusted, constructs a [`proxy::ProxyServer`],
/// and enters the TCP accept loop. Returns only on unrecoverable error.
pub async fn run(config: ProxyConfig) -> anyhow::Result<()> {
    let ca = tls::CaStore::load_or_create(&config.ca_dir).await?;

    #[cfg(target_os = "macos")]
    if !ca.is_installed()? {
        tracing::info!("CA not yet trusted — installing into macOS System Keychain");
        ca.install()?;
        tracing::info!("CA installed successfully");
    }

    let server = proxy::ProxyServer::new(config, ca);
    server.run().await?;
    Ok(())
}
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo test -p aa-proxy 2>&1
```

Expected: all non-`#[ignore]` tests pass. `clippy` must also be clean:

```bash
cargo clippy -p aa-proxy -- -D warnings 2>&1 | tail -20
```

Expected: no warnings or errors.

- [ ] **Step 3: Commit**

```bash
git add aa-proxy/src/lib.rs
git commit -m "✨ (lib): Wire CA is_installed + install into run() on macOS"
```
