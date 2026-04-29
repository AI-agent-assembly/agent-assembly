//! `aa-gateway` — Agent Assembly governance gateway gRPC server.

use std::path::PathBuf;

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

    if let Some(socket_path) = &cli.socket {
        aa_gateway::server::serve_uds(&cli.policy, socket_path).await
    } else {
        aa_gateway::server::serve_tcp(&cli.policy, &cli.listen).await
    }
}
