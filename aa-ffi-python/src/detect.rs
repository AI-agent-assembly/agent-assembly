//! Framework auto-detection via Python `sys.modules` probing.
//!
//! Checks which AI frameworks are already imported in the current Python
//! process. Returns a list of detected framework names. Does **not** install
//! hooks or perform monkey-patching — that is handled by the adapter registry
//! (AAASM-49) and per-framework adapter tickets (AAASM-50–53).

use pyo3::prelude::*;

/// Known AI frameworks to probe for.
const FRAMEWORK_MODULES: &[(&str, &str)] = &[
    ("langchain", "langchain"),
    ("langchain_core", "langchain"),
    ("langgraph", "langgraph"),
    ("crewai", "crewai"),
    ("pydantic_ai", "pydantic_ai"),
    ("autogen", "autogen"),
    ("openai", "openai"),
    ("anthropic", "anthropic"),
];

/// Detect which AI frameworks are loaded in the current Python process.
///
/// Probes `sys.modules` for known framework module names. Returns a
/// deduplicated list of detected framework names (e.g., `["langchain", "openai"]`).
pub fn detect_frameworks(py: Python<'_>) -> PyResult<Vec<String>> {
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;

    let mut detected: Vec<String> = Vec::new();

    for &(module_name, framework_name) in FRAMEWORK_MODULES {
        if modules.contains(module_name)? {
            let name = framework_name.to_string();
            if !detected.contains(&name) {
                detected.push(name);
            }
        }
    }

    Ok(detected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_modules_list_is_nonempty() {
        assert!(!FRAMEWORK_MODULES.is_empty());
    }

    #[test]
    fn framework_modules_entries_are_valid() {
        for &(module_name, framework_name) in FRAMEWORK_MODULES {
            assert!(!module_name.is_empty());
            assert!(!framework_name.is_empty());
        }
    }
}
