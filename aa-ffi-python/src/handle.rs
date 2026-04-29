//! Python-facing `AssemblyHandle` — the return type of `init_assembly()`.
//!
//! Manages the lifecycle of the IPC connection to `aa-runtime`. Supports
//! the Python context manager protocol (`with init_assembly() as handle:`).

use std::sync::Mutex;

use pyo3::prelude::*;

use crate::ipc::{IpcCommand, IpcHandle};

/// Handle to an active Agent Assembly session.
///
/// Returned by `init_assembly()`. Provides methods to report events to the
/// runtime and to shut down the connection. Supports the Python context
/// manager protocol.
///
/// ```python
/// with init_assembly(agent_id="my-agent") as handle:
///     handle.report_event("tool_call", {"tool": "search"})
/// # connection is cleaned up automatically
/// ```
#[pyclass]
pub struct AssemblyHandle {
    inner: Mutex<Option<IpcHandle>>,
    detected_frameworks: Vec<String>,
}

impl AssemblyHandle {
    /// Create a new handle wrapping an IPC connection.
    pub fn new(ipc_handle: IpcHandle, detected_frameworks: Vec<String>) -> Self {
        Self {
            inner: Mutex::new(Some(ipc_handle)),
            detected_frameworks,
        }
    }
}

#[pymethods]
impl AssemblyHandle {
    /// Report an audit event to the runtime.
    ///
    /// Args:
    ///     event_type: The type of event (e.g., "tool_call", "llm_response").
    ///     details: Human-readable description of the event.
    pub fn report_event(&self, event_type: String, details: String) -> PyResult<()> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("lock poisoned: {e}")))?;

        let ipc = guard.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("AssemblyHandle is shut down; cannot report events")
        })?;

        let mut labels = std::collections::HashMap::new();
        labels.insert("event_type".to_string(), event_type);
        labels.insert("details".to_string(), details);

        let event = aa_proto::assembly::audit::v1::AuditEvent {
            event_id: unique_event_id(),
            labels,
            ..Default::default()
        };

        ipc.cmd_tx
            .blocking_send(IpcCommand::SendEvent(event))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("failed to enqueue event: {e}")))?;

        Ok(())
    }

    /// Shut down the IPC connection and join the background thread.
    ///
    /// Safe to call multiple times — subsequent calls are no-ops.
    pub fn shutdown(&self, py: Python<'_>) -> PyResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("lock poisoned: {e}")))?;

        if let Some(mut ipc) = guard.take() {
            // Send shutdown command (best-effort — channel may be closed).
            let _ = ipc.cmd_tx.blocking_send(IpcCommand::Shutdown);

            // Join the background thread, releasing the GIL to avoid deadlock.
            if let Some(thread) = ipc.thread.take() {
                py.allow_threads(|| {
                    let _ = thread.join();
                });
            }
        }

        Ok(())
    }

    /// Returns the list of detected AI frameworks.
    pub fn detected_frameworks(&self) -> Vec<String> {
        self.detected_frameworks.clone()
    }

    /// Context manager entry — returns `self`.
    pub fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager exit — calls `shutdown()`.
    pub fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.shutdown(py)?;
        Ok(false) // Do not suppress exceptions.
    }
}

/// Generate a unique event ID string.
fn unique_event_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{:016x}-{:08x}-{:04x}", nanos, pid, seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_event_id_is_nonempty() {
        let id = unique_event_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn unique_event_id_unique() {
        let a = unique_event_id();
        let b = unique_event_id();
        // Not strictly guaranteed but extremely likely with nanos.
        assert_ne!(a, b);
    }
}
