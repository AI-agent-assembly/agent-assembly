//! TLS subsystem: CA management and per-domain certificate caching.

pub mod ca;
pub mod cert;
#[cfg(target_os = "macos")]
mod keychain;

pub use ca::{CaStore, CertifiedKey};
pub use cert::CertCache;
