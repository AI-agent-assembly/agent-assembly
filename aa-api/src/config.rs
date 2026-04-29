//! API server configuration.

use std::net::SocketAddr;

use crate::auth::config::{AuthConfig, AuthConfigError};

/// Default bind address for the API server.
const DEFAULT_ADDR: &str = "127.0.0.1:7700";

/// Configuration for the `aa-api` HTTP server.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Socket address to bind the server to.
    pub bind_addr: SocketAddr,
    /// Authentication configuration (fail-closed).
    pub auth: AuthConfig,
}

/// Errors that can occur when building `ApiConfig`.
#[derive(Debug, thiserror::Error)]
pub enum ApiConfigError {
    #[error("auth configuration error: {0}")]
    Auth(#[from] AuthConfigError),
}

impl ApiConfig {
    /// Build configuration from environment variables.
    ///
    /// Reads `AA_API_ADDR` (e.g. `"0.0.0.0:7700"`). Falls back to
    /// `127.0.0.1:7700` when the variable is unset.
    ///
    /// Also reads auth-related env vars via [`AuthConfig::from_env()`].
    /// If auth is enabled and configuration is invalid, this returns an
    /// error (fail-closed).
    pub fn from_env() -> Result<Self, ApiConfigError> {
        let addr = std::env::var("AA_API_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
        let bind_addr = addr.parse().unwrap_or_else(|e| {
            tracing::warn!(
                addr = %addr,
                error = %e,
                "invalid AA_API_ADDR, falling back to default"
            );
            DEFAULT_ADDR.parse().expect("default address is valid")
        });
        let auth = AuthConfig::from_env()?;
        Ok(Self { bind_addr, auth })
    }
}
