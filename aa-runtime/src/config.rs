//! Runtime configuration loaded from environment variables.

/// Configuration for the `aa-runtime` sidecar process.
///
/// All fields are populated by [`RuntimeConfig::from_env`].
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Stable identity of this agent instance.
    ///
    /// Read from `AA_AGENT_ID`. Required — startup fails if unset.
    /// Used to name the Unix socket: `/tmp/aa-runtime-<agent_id>.sock`.
    pub agent_id: String,

    /// Number of Tokio worker threads.
    ///
    /// Read from `AA_RUNTIME_WORKER_THREADS`. Defaults to `0`, which tells
    /// Tokio to use one thread per logical CPU.
    pub worker_threads: usize,

    /// Maximum seconds to wait for in-flight tasks to complete during shutdown.
    ///
    /// Read from `AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS`. Defaults to `30`.
    pub shutdown_timeout_secs: u64,

    /// Maximum number of concurrent SDK connections to the IPC socket.
    ///
    /// Read from `AA_IPC_MAX_CONNECTIONS`. Defaults to `64`.
    pub ipc_max_connections: usize,
}

impl RuntimeConfig {
    /// Build configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if `AA_AGENT_ID` is not set.
    ///
    /// # Env vars
    ///
    /// | Variable | Type | Default |
    /// |---|---|---|
    /// | `AA_AGENT_ID` | `String` | **required** |
    /// | `AA_RUNTIME_WORKER_THREADS` | `usize` | `0` (Tokio picks per-CPU) |
    /// | `AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS` | `u64` | `30` |
    /// | `AA_IPC_MAX_CONNECTIONS` | `usize` | `64` |
    pub fn from_env() -> Result<Self, String> {
        let agent_id = std::env::var("AA_AGENT_ID").map_err(|_| "AA_AGENT_ID is required but not set".to_string())?;

        if agent_id.trim().is_empty() {
            return Err("AA_AGENT_ID must not be blank or empty".to_string());
        }

        if agent_id.contains('/') || agent_id.contains("..") {
            return Err("AA_AGENT_ID must not contain path separators ('/' or '..')".to_string());
        }

        let worker_threads = std::env::var("AA_RUNTIME_WORKER_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let shutdown_timeout_secs = std::env::var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let ipc_max_connections = std::env::var("AA_IPC_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(64);

        Ok(Self {
            agent_id,
            worker_threads,
            shutdown_timeout_secs,
            ipc_max_connections,
        })
    }
}

#[cfg(test)]
mod tests {
    //! # Test isolation requirement
    //!
    //! These tests mutate process environment variables and must be run sequentially:
    //! ```text
    //! cargo test -p aa-runtime -- --test-threads=1
    //! ```
    //! Running with the default thread pool causes env var races between tests.

    use super::*;
    use std::sync::Mutex;

    // Env vars are process-global; this mutex serializes all tests that
    // read or write them so they cannot race under multi-threaded test runners
    // (e.g. `cargo llvm-cov` which uses `cargo test` with parallel threads).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn reads_agent_id_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "test-agent-42");
        std::env::remove_var("AA_RUNTIME_WORKER_THREADS");
        std::env::remove_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");

        let config = RuntimeConfig::from_env().expect("should succeed with AA_AGENT_ID set");

        assert_eq!(config.agent_id, "test-agent-42");
        assert_eq!(config.worker_threads, 0);
        assert_eq!(config.shutdown_timeout_secs, 30);
        assert_eq!(config.ipc_max_connections, 64);

        std::env::remove_var("AA_AGENT_ID");
    }

    #[test]
    fn fails_fast_when_agent_id_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("AA_AGENT_ID");

        let result = RuntimeConfig::from_env();

        assert!(result.is_err(), "expected error when AA_AGENT_ID is not set");
        assert!(result.unwrap_err().contains("AA_AGENT_ID"));
    }

    #[test]
    fn fails_fast_when_agent_id_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "   ");

        let result = RuntimeConfig::from_env();

        assert!(result.is_err());

        std::env::remove_var("AA_AGENT_ID");
    }

    #[test]
    fn defaults_when_env_vars_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "default-test-agent");
        std::env::remove_var("AA_RUNTIME_WORKER_THREADS");
        std::env::remove_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.worker_threads, 0);
        assert_eq!(config.shutdown_timeout_secs, 30);
        assert_eq!(config.ipc_max_connections, 64);

        std::env::remove_var("AA_AGENT_ID");
    }

    #[test]
    fn reads_worker_threads_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "agent-wt");
        std::env::set_var("AA_RUNTIME_WORKER_THREADS", "4");
        std::env::remove_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.worker_threads, 4);
        assert_eq!(config.shutdown_timeout_secs, 30);

        std::env::remove_var("AA_AGENT_ID");
        std::env::remove_var("AA_RUNTIME_WORKER_THREADS");
    }

    #[test]
    fn reads_shutdown_timeout_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "agent-st");
        std::env::remove_var("AA_RUNTIME_WORKER_THREADS");
        std::env::set_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS", "60");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.worker_threads, 0);
        assert_eq!(config.shutdown_timeout_secs, 60);

        std::env::remove_var("AA_AGENT_ID");
        std::env::remove_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS");
    }

    #[test]
    fn reads_ipc_max_connections_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "agent-mc");
        std::env::set_var("AA_IPC_MAX_CONNECTIONS", "128");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.ipc_max_connections, 128);

        std::env::remove_var("AA_AGENT_ID");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");
    }

    #[test]
    fn rejects_zero_ipc_max_connections() {
        std::env::set_var("AA_AGENT_ID", "agent-zero");
        std::env::set_var("AA_IPC_MAX_CONNECTIONS", "0");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.ipc_max_connections, 64, "0 should fall back to default");

        std::env::remove_var("AA_AGENT_ID");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");
    }

    #[test]
    fn rejects_agent_id_with_path_separator() {
        std::env::set_var("AA_AGENT_ID", "../../etc/passwd");

        let result = RuntimeConfig::from_env();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path separator"));

        std::env::remove_var("AA_AGENT_ID");
    }

    #[test]
    fn falls_back_to_default_on_invalid_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AA_AGENT_ID", "agent-inv");
        std::env::set_var("AA_RUNTIME_WORKER_THREADS", "not-a-number");
        std::env::set_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS", "abc");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");

        let config = RuntimeConfig::from_env().unwrap();

        assert_eq!(config.worker_threads, 0);
        assert_eq!(config.shutdown_timeout_secs, 30);
        assert_eq!(config.ipc_max_connections, 64);

        std::env::remove_var("AA_AGENT_ID");
        std::env::remove_var("AA_RUNTIME_WORKER_THREADS");
        std::env::remove_var("AA_RUNTIME_SHUTDOWN_TIMEOUT_SECS");
        std::env::remove_var("AA_IPC_MAX_CONNECTIONS");
    }
}
