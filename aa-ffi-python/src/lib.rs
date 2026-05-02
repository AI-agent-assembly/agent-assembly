//! Python FFI bindings for Agent Assembly via PyO3.
//!
//! This crate exposes the Agent Assembly SDK to Python. It compiles to a
//! `cdylib` Python extension module, allowing Python agents to instrument
//! themselves with the governance shim without leaving the Python runtime.
//!
//! # Usage
//!
//! ```python
//! from agent_assembly import init_assembly
//!
//! with init_assembly(agent_id="my-agent") as handle:
//!     print(handle.detected_frameworks())
//!     handle.report_event("tool_call", "searched for documents")
//! # connection is cleaned up automatically
//! ```

mod codec;
mod config;
mod detect;
mod handle;
mod hooks;
mod ipc;

use pyo3::prelude::*;

use config::AssemblyConfig;
use handle::AssemblyHandle;

/// Initialize an Agent Assembly session.
///
/// Connects to the `aa-runtime` governance sidecar over a Unix domain socket,
/// performs framework auto-detection, and returns an `AssemblyHandle` that can
/// be used as a Python context manager.
///
/// Args:
///     agent_id: Unique identifier for this agent instance.
///     socket_path: Optional explicit path to the runtime socket. If omitted,
///         resolved from the `AA_RUNTIME_SOCKET` env var or the default
///         `/tmp/aa-runtime-<agent_id>.sock`.
///     mode: Operating mode — `"auto"` (default) connects to the runtime;
///         `"embedded"` is reserved for future use and currently raises an error.
///
/// Returns:
///     An `AssemblyHandle` that supports the context manager protocol.
///
/// Raises:
///     ValueError: If `agent_id` is empty.
///     RuntimeError: If `mode` is `"embedded"` (not yet supported).
///     OSError: If the background IPC thread cannot be spawned.
#[pyfunction]
#[pyo3(signature = (agent_id, socket_path = None, mode = "auto"))]
fn init_assembly(
    py: Python<'_>,
    agent_id: String,
    socket_path: Option<String>,
    mode: &str,
) -> PyResult<Py<AssemblyHandle>> {
    // Validate agent_id.
    if agent_id.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err("agent_id must not be empty"));
    }

    // Validate mode.
    if mode == "embedded" {
        return Err(pyo3::exceptions::PyRuntimeError::new_err(
            "embedded mode is not yet supported; use mode=\"auto\" (the default)",
        ));
    }

    if mode != "auto" {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown mode \"{mode}\"; expected \"auto\" or \"embedded\""
        )));
    }

    // Build configuration.
    let config = AssemblyConfig { agent_id, socket_path };

    // Detect loaded AI frameworks.
    let detected_frameworks = detect::detect_frameworks(py)?;

    // Resolve socket path and spawn the background IPC thread.
    let resolved_path = config.resolve_socket_path();
    let ipc_handle = ipc::spawn_ipc_thread(resolved_path)
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("failed to spawn IPC thread: {e}")))?;

    // Create the Python handle object so we can pass it to hook installers.
    let handle = Py::new(py, AssemblyHandle::new(ipc_handle, detected_frameworks.clone()))?;

    // Install framework-specific hooks for each detected framework.
    let _installed = hooks::install_hooks(py, handle.bind(py), &detected_frameworks);

    Ok(handle)
}

/// Python module definition for `agent_assembly`.
#[pymodule]
fn agent_assembly(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init_assembly, m)?)?;
    m.add_class::<AssemblyHandle>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_assembly_rejects_empty_agent_id() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let result = init_assembly(py, String::new(), None, "auto");
            let err = result.expect_err("should reject empty agent_id");
            assert!(err.to_string().contains("agent_id must not be empty"));
        });
    }

    #[test]
    fn init_assembly_rejects_embedded_mode() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let result = init_assembly(py, "test-agent".to_string(), None, "embedded");
            let err = result.expect_err("should reject embedded mode");
            assert!(err.to_string().contains("embedded mode is not yet supported"));
        });
    }

    #[test]
    fn init_assembly_rejects_unknown_mode() {
        pyo3::Python::initialize();
        Python::attach(|py| {
            let result = init_assembly(py, "test-agent".to_string(), None, "bogus");
            let err = result.expect_err("should reject unknown mode");
            assert!(err.to_string().contains("unknown mode"));
        });
    }

    #[test]
    fn init_assembly_auto_mode_spawns_handle() {
        // In auto mode with a valid agent_id, init_assembly should succeed
        // even though the socket doesn't exist — the IPC thread spawns and
        // will fail to connect asynchronously.
        pyo3::Python::initialize();
        Python::attach(|py| {
            let result = init_assembly(
                py,
                "test-agent".to_string(),
                Some("/tmp/nonexistent-aa-test.sock".to_string()),
                "auto",
            );
            assert!(result.is_ok());
            let handle = result.unwrap();
            // Clean up the background thread.
            handle.bind(py).borrow().shutdown(py).unwrap();
        });
    }
}
