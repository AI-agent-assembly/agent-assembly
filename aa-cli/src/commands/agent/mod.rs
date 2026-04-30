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
