//! `aasm run` — launch an AI dev tool with governance wiring.

use std::collections::HashMap;
use std::process::ExitCode;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use aa_core::{AdapterError, DevToolAdapter, DevToolInfo, GovernanceLevel, McpServerInfo, PolicyDocument};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

/// Arguments for the `aasm run <tool> [args...]` subcommand.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// The AI development tool to launch (claude, codex, copilot, windsurf).
    pub tool: String,

    /// Arguments forwarded verbatim to the launched tool.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub tool_args: Vec<String>,

    /// Override the agent identity for this session.
    #[arg(long)]
    pub agent_id: Option<String>,

    /// Team identifier for this session.
    #[arg(long)]
    pub team_id: Option<String>,

    /// Root agent identifier for lineage tracking.
    #[arg(long)]
    pub root_agent: Option<String>,

    /// Override the governance level for this session.
    #[arg(long)]
    pub governance_level: Option<GovernanceLevel>,

    /// Skip proxy injection (not recommended for governed environments).
    #[arg(long)]
    pub no_proxy: bool,

    /// Show the launch command and settings without executing.
    #[arg(long)]
    pub dry_run: bool,
}

// Placeholder until per-tool adapter crates (AAASM-201..205) are ready.
// Each of the four known tools maps to this struct; replace individual arms
// in resolve_adapter() when the real crate lands.
struct PlaceholderAdapter;

#[async_trait]
impl DevToolAdapter for PlaceholderAdapter {
    fn detect(&self) -> Option<DevToolInfo> {
        None
    }

    async fn generate_managed_settings(&self, _policy: &PolicyDocument) -> Result<String, AdapterError> {
        Err(AdapterError::SettingsGenerationFailed(
            "adapter not yet implemented".into(),
        ))
    }

    async fn apply_settings(&self, _settings: &str) -> Result<(), AdapterError> {
        Err(AdapterError::SettingsApplyFailed(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "adapter not yet implemented",
        )))
    }

    fn build_launch_command(
        &self,
        _tool_args: &[String],
        _agent_id: &str,
        _team_id: Option<&str>,
        _proxy_addr: Option<&str>,
    ) -> Result<std::process::Command, AdapterError> {
        Err(AdapterError::LaunchFailed("adapter not yet implemented".into()))
    }

    async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AdapterError> {
        Ok(vec![])
    }

    async fn apply_mcp_governance(&self, _allowed: &[String], _denied: &[String]) -> Result<(), AdapterError> {
        Ok(())
    }

    fn governance_level(&self) -> GovernanceLevel {
        GovernanceLevel::L0Discover
    }
}

/// Return the adapter for `tool`, or an error for unrecognised tool names.
///
/// Maps the four supported tool names to their adapter implementations.
/// Returns [`Err`] with a human-readable message for any other value so the
/// CLI can surface a clean "unknown tool" error before touching the filesystem.
fn resolve_adapter(tool: &str) -> Result<Box<dyn DevToolAdapter>> {
    match tool {
        // Real adapters replace PlaceholderAdapter here once their crates land.
        "claude" => Ok(Box::new(PlaceholderAdapter)),
        "codex" => Ok(Box::new(PlaceholderAdapter)),
        "copilot" => Ok(Box::new(PlaceholderAdapter)),
        "windsurf" => Ok(Box::new(PlaceholderAdapter)),
        _ => Err(anyhow::anyhow!(
            "unknown tool: {tool}, supported: claude, codex, copilot, windsurf"
        )),
    }
}

/// Testable core of `execute`: look up the adapter, run detection, print summary.
///
/// Accepts an explicit adapter map so tests can inject stubs without touching
/// the filesystem or requiring real tool binaries.
async fn execute_with_adapters(args: &RunArgs, adapters: &HashMap<&str, Box<dyn DevToolAdapter>>) -> Result<()> {
    let adapter = adapters.get(args.tool.as_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown tool: {}, supported: claude, codex, copilot, windsurf",
            args.tool
        )
    })?;

    let info = adapter
        .detect()
        .ok_or_else(|| anyhow::anyhow!("{} is not installed", args.tool))?;

    eprintln!(
        "tool={} version={} path={} governance_level={}",
        args.tool,
        info.version.as_deref().unwrap_or("unknown"),
        info.install_path.display(),
        info.governance_level,
    );

    // Subsequent subtasks (AAASM-935+) add registration and exec here.
    Ok(())
}

/// Launch the specified AI dev tool with governance wiring.
pub async fn execute(args: RunArgs, _ctx: &ResolvedContext) -> Result<()> {
    let mut adapters: HashMap<&str, Box<dyn DevToolAdapter>> = HashMap::new();
    for tool in ["claude", "codex", "copilot", "windsurf"] {
        adapters.insert(tool, resolve_adapter(tool)?);
    }
    execute_with_adapters(&args, &adapters).await
}

/// Entry point for `aasm run`.
pub fn dispatch(args: RunArgs, ctx: &ResolvedContext, _output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(execute(args, ctx)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use aa_core::{DevToolInfo, DevToolKind};
    use clap::Parser;

    use super::*;

    /// Minimal CLI wrapper for testing `run` subcommand parsing.
    #[derive(Parser)]
    #[command(name = "aasm")]
    struct TestCli {
        #[command(subcommand)]
        command: TestCommands,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        Run(RunArgs),
    }

    // --- parse tests (carried forward from AAASM-927) ---

    #[test]
    fn parse_basic_run_command() {
        let cli = TestCli::try_parse_from(["aasm", "run", "claude", "foo", "bar"]).unwrap();
        match cli.command {
            TestCommands::Run(args) => {
                assert_eq!(args.tool, "claude");
                assert_eq!(args.tool_args, vec!["foo", "bar"]);
                assert!(!args.dry_run);
                assert!(!args.no_proxy);
            }
        }
    }

    #[test]
    fn parse_with_flags() {
        let cli = TestCli::try_parse_from([
            "aasm",
            "run",
            "claude",
            "--agent-id",
            "a1",
            "--dry-run",
            "--",
            "--some-flag",
        ])
        .unwrap();
        match cli.command {
            TestCommands::Run(args) => {
                assert_eq!(args.tool, "claude");
                assert_eq!(args.agent_id.as_deref(), Some("a1"));
                assert!(args.dry_run);
                assert_eq!(args.tool_args, vec!["--some-flag"]);
            }
        }
    }

    #[test]
    fn parse_governance_level_short_forms() {
        for (input, expected) in [
            ("L0", GovernanceLevel::L0Discover),
            ("L1", GovernanceLevel::L1Observe),
            ("L2", GovernanceLevel::L2Enforce),
            ("L3", GovernanceLevel::L3Native),
        ] {
            let cli = TestCli::try_parse_from(["aasm", "run", "codex", "--governance-level", input]).unwrap();
            match cli.command {
                TestCommands::Run(args) => {
                    assert_eq!(args.governance_level, Some(expected), "input={input}");
                }
            }
        }
    }

    // --- adapter resolution tests ---

    #[test]
    fn unknown_tool_errors() {
        // Box<dyn DevToolAdapter> is not Debug, so use match instead of unwrap_err().
        let err = match resolve_adapter("notathing") {
            Ok(_) => panic!("expected Err for unknown tool"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("unknown tool"),
            "expected 'unknown tool' in error, got: {err}"
        );
        assert!(
            err.to_string().contains("notathing"),
            "expected tool name in error, got: {err}"
        );
    }

    #[test]
    fn known_tools_resolve_without_error() {
        for tool in ["claude", "codex", "copilot", "windsurf"] {
            assert!(resolve_adapter(tool).is_ok(), "resolve_adapter({tool}) should succeed");
        }
    }

    // --- execute_with_adapters tests ---

    /// Stub adapter whose detect() always returns None (tool not installed).
    struct StubNotInstalled;

    #[async_trait]
    impl DevToolAdapter for StubNotInstalled {
        fn detect(&self) -> Option<DevToolInfo> {
            None
        }
        async fn generate_managed_settings(&self, _p: &PolicyDocument) -> Result<String, AdapterError> {
            unimplemented!()
        }
        async fn apply_settings(&self, _s: &str) -> Result<(), AdapterError> {
            unimplemented!()
        }
        fn build_launch_command(
            &self,
            _a: &[String],
            _b: &str,
            _c: Option<&str>,
            _d: Option<&str>,
        ) -> Result<std::process::Command, AdapterError> {
            unimplemented!()
        }
        async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AdapterError> {
            unimplemented!()
        }
        async fn apply_mcp_governance(&self, _a: &[String], _d: &[String]) -> Result<(), AdapterError> {
            unimplemented!()
        }
        fn governance_level(&self) -> GovernanceLevel {
            GovernanceLevel::L0Discover
        }
    }

    /// Stub adapter whose detect() returns a synthetic DevToolInfo.
    struct StubDetected {
        version: Option<String>,
    }

    #[async_trait]
    impl DevToolAdapter for StubDetected {
        fn detect(&self) -> Option<DevToolInfo> {
            Some(DevToolInfo {
                kind: DevToolKind::ClaudeCode,
                version: self.version.clone(),
                install_path: PathBuf::from("/usr/local/bin/claude"),
                governance_level: GovernanceLevel::L2Enforce,
                supports_mcp: true,
                supports_managed_settings: true,
            })
        }
        async fn generate_managed_settings(&self, _p: &PolicyDocument) -> Result<String, AdapterError> {
            unimplemented!()
        }
        async fn apply_settings(&self, _s: &str) -> Result<(), AdapterError> {
            unimplemented!()
        }
        fn build_launch_command(
            &self,
            _a: &[String],
            _b: &str,
            _c: Option<&str>,
            _d: Option<&str>,
        ) -> Result<std::process::Command, AdapterError> {
            unimplemented!()
        }
        async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, AdapterError> {
            unimplemented!()
        }
        async fn apply_mcp_governance(&self, _a: &[String], _d: &[String]) -> Result<(), AdapterError> {
            unimplemented!()
        }
        fn governance_level(&self) -> GovernanceLevel {
            GovernanceLevel::L2Enforce
        }
    }

    fn run_args(tool: &str) -> RunArgs {
        RunArgs {
            tool: tool.to_string(),
            tool_args: vec![],
            agent_id: None,
            team_id: None,
            root_agent: None,
            governance_level: None,
            no_proxy: false,
            dry_run: false,
        }
    }

    #[tokio::test]
    async fn tool_not_found_errors() {
        let mut adapters: HashMap<&str, Box<dyn DevToolAdapter>> = HashMap::new();
        adapters.insert("claude", Box::new(StubNotInstalled));

        let err = execute_with_adapters(&run_args("claude"), &adapters).await.unwrap_err();
        assert!(
            err.to_string().contains("is not installed"),
            "expected 'is not installed' in error, got: {err}"
        );
        assert!(
            err.to_string().contains("claude"),
            "expected tool name in error, got: {err}"
        );
    }

    #[tokio::test]
    async fn detected_tool_succeeds() {
        let mut adapters: HashMap<&str, Box<dyn DevToolAdapter>> = HashMap::new();
        adapters.insert(
            "claude",
            Box::new(StubDetected {
                version: Some("1.2.3".into()),
            }),
        );

        assert!(
            execute_with_adapters(&run_args("claude"), &adapters).await.is_ok(),
            "execute_with_adapters should succeed when detect() returns Some"
        );
    }

    #[tokio::test]
    async fn unknown_tool_in_adapters_errors() {
        let adapters: HashMap<&str, Box<dyn DevToolAdapter>> = HashMap::new();

        let err = execute_with_adapters(&run_args("notathing"), &adapters)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("unknown tool"), "got: {err}");
    }
}
