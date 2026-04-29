//! `aa-gateway` — Agent Assembly governance gateway gRPC server.

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Agent Assembly governance gateway — gRPC policy evaluation server.
#[derive(Parser)]
#[command(name = "aa-gateway", version, about)]
struct Cli {
    /// Path to the policy YAML file.
    #[arg(long)]
    policy: PathBuf,

    /// TCP listen address (e.g. "127.0.0.1:50051").
    #[arg(long, default_value = "127.0.0.1:50051")]
    listen: String,

    /// Unix domain socket path. When set, takes precedence over --listen.
    #[arg(long)]
    socket: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    tracing::info!(policy = %cli.policy.display(), "loading policy");

    let registry = Arc::new(aa_gateway::AgentRegistry::new());

    // Create the approval queue — gateway-owned, shared with the runtime via gRPC.
    let approval_queue = aa_runtime::approval::ApprovalQueue::new();

    // Create a budget alert broadcast channel shared between the PolicyEngine
    // (sender, via BudgetTracker) and the webhook delivery loop (receiver).
    let (budget_alert_tx, budget_alert_rx) =
        tokio::sync::broadcast::channel::<aa_gateway::budget::BudgetAlert>(64);

    // Optionally spawn the webhook delivery loop (reads AA_WEBHOOK_URL).
    let _webhook_handle = aa_gateway::events::startup::maybe_spawn_webhook(&approval_queue, budget_alert_rx);

    if let Some(socket_path) = &cli.socket {
        aa_gateway::server::serve_uds(&cli.policy, socket_path, registry, approval_queue, budget_alert_tx).await
    } else {
        aa_gateway::server::serve_tcp(&cli.policy, &cli.listen, registry, approval_queue, budget_alert_tx).await
    }
}
