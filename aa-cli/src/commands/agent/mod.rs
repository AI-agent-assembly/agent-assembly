//! `aasm agent` — manage monitored agent processes.

use std::collections::BTreeMap;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

mod inspect;
mod kill;
mod list;

/// Arguments for the `aasm agent` subcommand group.
#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommands,
}

/// Available agent subcommands.
#[derive(Subcommand)]
pub enum AgentCommands {
    /// List all registered agents.
    List(list::ListArgs),
    /// Show detailed information about a specific agent.
    Inspect(inspect::InspectArgs),
    /// Deregister and terminate an agent.
    Kill(kill::KillArgs),
}

/// Dispatch an agent subcommand.
pub fn dispatch(args: AgentArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    match args.command {
        AgentCommands::List(list_args) => list::run(list_args, ctx, output),
        AgentCommands::Inspect(inspect_args) => inspect::run(inspect_args, ctx, output),
        AgentCommands::Kill(kill_args) => kill::run(kill_args, ctx),
    }
}

/// JSON representation of an agent returned by the gateway API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Hex-encoded agent UUID.
    pub id: String,
    /// Human-readable agent name.
    pub name: String,
    /// Agent framework (e.g. "langgraph", "crewai").
    pub framework: String,
    /// Semver version string.
    pub version: String,
    /// Current runtime status.
    pub status: String,
    /// Tools declared at registration.
    pub tool_names: Vec<String>,
    /// Arbitrary metadata key-value pairs.
    pub metadata: BTreeMap<String, String>,
}

/// Paginated API response wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    #[allow(dead_code)]
    pub page: u32,
    #[allow(dead_code)]
    pub per_page: u32,
    #[allow(dead_code)]
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_response_deserializes_from_json() {
        let json = r#"{
            "id": "aabbccdd00112233aabbccdd00112233",
            "name": "my-agent",
            "framework": "langgraph",
            "version": "0.2.0",
            "status": "Active",
            "tool_names": ["search", "calculator"],
            "metadata": {"env": "production"}
        }"#;

        let agent: AgentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(agent.id, "aabbccdd00112233aabbccdd00112233");
        assert_eq!(agent.name, "my-agent");
        assert_eq!(agent.framework, "langgraph");
        assert_eq!(agent.version, "0.2.0");
        assert_eq!(agent.status, "Active");
        assert_eq!(agent.tool_names, vec!["search", "calculator"]);
        assert_eq!(agent.metadata.get("env").unwrap(), "production");
    }

    #[test]
    fn agent_response_round_trip() {
        let agent = AgentResponse {
            id: "00112233445566778899aabbccddeeff".to_string(),
            name: "round-trip-agent".to_string(),
            framework: "crewai".to_string(),
            version: "1.0.0".to_string(),
            status: "Suspended(PolicyViolation)".to_string(),
            tool_names: vec![],
            metadata: BTreeMap::new(),
        };

        let json = serde_json::to_string(&agent).unwrap();
        let parsed: AgentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, agent.id);
        assert_eq!(parsed.status, agent.status);
        assert!(parsed.tool_names.is_empty());
    }

    #[test]
    fn paginated_response_deserializes() {
        let json = r#"{
            "items": [
                {
                    "id": "aabb",
                    "name": "a1",
                    "framework": "f1",
                    "version": "0.1.0",
                    "status": "Active",
                    "tool_names": [],
                    "metadata": {}
                }
            ],
            "page": 1,
            "per_page": 20,
            "total": 1
        }"#;

        let resp: PaginatedResponse<AgentResponse> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].name, "a1");
        assert_eq!(resp.page, 1);
        assert_eq!(resp.total, 1);
    }

    #[test]
    fn agent_response_with_empty_metadata() {
        let json = r#"{
            "id": "ff",
            "name": "empty-meta",
            "framework": "custom",
            "version": "0.0.1",
            "status": "Deregistered",
            "tool_names": [],
            "metadata": {}
        }"#;

        let agent: AgentResponse = serde_json::from_str(json).unwrap();
        assert!(agent.metadata.is_empty());
        assert!(agent.tool_names.is_empty());
    }
}
