//! LRU cache for dynamically generated per-domain TLS certificates.
//!
//! Generating a certificate with rcgen takes ~1–2 ms. This cache avoids
//! regenerating a cert for every connection to the same domain.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use lru::LruCache;

use crate::error::ProxyError;
use crate::tls::ca::{CaStore, CertifiedKey};

/// Thread-safe LRU cache mapping domain names to their signed [`CertifiedKey`].
// Field is read by get_or_insert() once implemented; silence dead_code until then.
#[allow(dead_code)]
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
