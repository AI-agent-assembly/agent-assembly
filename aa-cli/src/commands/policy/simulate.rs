//! `aasm policy simulate` — dry-run policy evaluation.

use std::path::PathBuf;

use clap::Args;

// Gateway types will be used when simulation logic is implemented.
#[allow(unused_imports)]
use aa_gateway::simulation::{SimulationEngine, SimulationReport};

/// Arguments for `aasm policy simulate`.
#[derive(Args)]
pub struct SimulateArgs {
    /// Path to the policy YAML file to simulate.
    #[arg(long)]
    pub policy: PathBuf,

    /// Path to an audit log JSONL file to replay against the policy.
    #[arg(long)]
    pub against: Option<PathBuf>,

    /// Observe live agent traffic instead of replaying a file.
    #[arg(long, default_value_t = false)]
    pub live: bool,

    /// Duration for live simulation (e.g. "60s", "5m").
    #[arg(long)]
    pub duration: Option<String>,

    /// Path to write the simulation report JSON.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

/// Execute the simulate command.
pub fn run(_args: SimulateArgs) {
    todo!("AAASM-73: implement policy simulation CLI handler")
}
