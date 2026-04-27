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
