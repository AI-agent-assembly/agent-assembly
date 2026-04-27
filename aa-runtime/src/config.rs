//! Runtime configuration loaded from environment variables.

/// Configuration for the `aa-runtime` sidecar process.
///
/// All fields are populated by [`RuntimeConfig::from_env`].
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of Tokio worker threads.
    ///
    /// Read from `AA_RUNTIME_WORKER_THREADS`. Defaults to `0`, which tells
    /// Tokio to use one thread per logical CPU.
    pub worker_threads: usize,

    /// Maximum seconds to wait for in-flight tasks to complete during shutdown.
    ///
    /// Read from `AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS`. Defaults to `30`.
    pub shutdown_timeout_secs: u64,
}

impl RuntimeConfig {
    /// Build configuration from environment variables, falling back to defaults.
    ///
    /// # Env vars
    ///
    /// | Variable | Type | Default |
    /// |---|---|---|
    /// | `AA_RUNTIME_WORKER_THREADS` | `usize` | `0` (Tokio picks per-CPU) |
    /// | `AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS` | `u64` | `30` |
    ///
    /// Invalid values are silently ignored and the default is used instead.
    pub fn from_env() -> Self {
        let worker_threads = std::env::var("AA_RUNTIME_WORKER_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let shutdown_timeout_secs = std::env::var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        Self {
            worker_threads,
            shutdown_timeout_secs,
        }
    }
}
