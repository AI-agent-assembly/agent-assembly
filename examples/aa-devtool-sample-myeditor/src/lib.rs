//! Sample [`DevToolAdapter`] implementation for a fictional `MyEditor` IDE.
//!
//! This crate exists as a **reference for plugin authors** (see
//! [`docs/devtools/plugins.md`]). It is intentionally hand-rolled — no
//! real `myeditor` binary exists. Detection succeeds when an env var
//! pointing at a stub binary is set; MCP-server discovery reads a
//! fixture JSON shipped under `fixtures/events.json`. Concrete per-tool
//! adapters (Claude Code, Codex, Copilot, Windsurf, SaaS) are tracked
//! separately in AAASM-201..205 and AAASM-918.
//!
//! [`DevToolAdapter`]: aa_core::DevToolAdapter
//! [`docs/devtools/plugins.md`]: https://github.com/AI-agent-assembly/agent-assembly/blob/master/docs/devtools/plugins.md

#![warn(missing_docs)]
