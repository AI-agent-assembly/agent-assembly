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
