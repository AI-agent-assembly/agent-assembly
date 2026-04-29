//! Authentication configuration from environment variables.

use std::path::PathBuf;

use thiserror::Error;

/// Default path for API keys storage.
const DEFAULT_API_KEYS_PATH: &str = "~/.aa/api-keys.json";

/// Default rate limit: requests per minute per API key.
const DEFAULT_RATE_LIMIT_RPM: u32 = 1000;

/// Minimum length for the JWT secret (256 bits).
const MIN_JWT_SECRET_LEN: usize = 32;

/// Authentication mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Authentication is enabled (default).
    On,
    /// Authentication is disabled — all requests are treated as admin.
    Off,
}

/// Authentication configuration for the API server.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether auth is enabled or bypassed.
    pub mode: AuthMode,
    /// HMAC-SHA256 secret for JWT signing. `None` when `mode == Off`.
    pub jwt_secret: Option<Vec<u8>>,
    /// Path to the API keys JSON file.
    pub api_keys_path: PathBuf,
    /// Maximum requests per minute per API key.
    pub rate_limit_rpm: u32,
}

/// Errors that can occur when loading auth configuration.
#[derive(Debug, Error)]
pub enum AuthConfigError {
    #[error("AA_JWT_SECRET must be set when authentication is enabled")]
    MissingJwtSecret,
    #[error(
        "AA_JWT_SECRET must be at least {MIN_JWT_SECRET_LEN} bytes (got {actual} bytes)"
    )]
    JwtSecretTooShort { actual: usize },
    #[error("AA_RATE_LIMIT_RPM must be a positive integer: {0}")]
    InvalidRateLimit(String),
}

impl AuthConfig {
    /// Build auth configuration from environment variables.
    ///
    /// # Environment variables
    ///
    /// - `AA_AUTH`: `"on"` (default) or `"off"` (bypass mode)
    /// - `AA_JWT_SECRET`: HMAC key for JWT, required when auth is enabled
    /// - `AA_API_KEYS_PATH`: path to API keys file (default `~/.aa/api-keys.json`)
    /// - `AA_RATE_LIMIT_RPM`: requests per minute per key (default 1000)
    pub fn from_env() -> Result<Self, AuthConfigError> {
        let mode = match std::env::var("AA_AUTH").as_deref() {
            Ok("off") | Ok("OFF") => {
                tracing::warn!("AA_AUTH=off: authentication is disabled — all requests treated as admin");
                AuthMode::Off
            }
            _ => AuthMode::On,
        };

        let jwt_secret = if mode == AuthMode::On {
            let secret = std::env::var("AA_JWT_SECRET")
                .map_err(|_| AuthConfigError::MissingJwtSecret)?;
            let bytes = secret.into_bytes();
            if bytes.len() < MIN_JWT_SECRET_LEN {
                return Err(AuthConfigError::JwtSecretTooShort {
                    actual: bytes.len(),
                });
            }
            Some(bytes)
        } else {
            None
        };

        let api_keys_path = std::env::var("AA_API_KEYS_PATH")
            .unwrap_or_else(|_| DEFAULT_API_KEYS_PATH.to_string());
        let api_keys_path = expand_tilde(&api_keys_path);

        let rate_limit_rpm = match std::env::var("AA_RATE_LIMIT_RPM") {
            Ok(val) => val
                .parse::<u32>()
                .map_err(|_| AuthConfigError::InvalidRateLimit(val))?,
            Err(_) => DEFAULT_RATE_LIMIT_RPM,
        };

        Ok(Self {
            mode,
            jwt_secret,
            api_keys_path,
            rate_limit_rpm,
        })
    }
}

/// Expand `~` prefix to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
