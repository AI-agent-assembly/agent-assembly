//! `aa-gateway` — Agent Assembly governance gateway gRPC server.
//!
//! Loads a policy YAML, starts the `PolicyEngine` with hot-reload, and serves
//! `PolicyService` over TCP (default) or Unix domain socket.

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use aa_gateway::engine::PolicyEngine;
use aa_gateway::service::PolicyServiceImpl;
use aa_proto::assembly::policy::v1::policy_service_server::PolicyServiceServer;

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
    // Initialise tracing (respects RUST_LOG env var, defaults to info).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    tracing::info!(policy = %cli.policy.display(), "loading policy");
    let engine = PolicyEngine::load_from_file(&cli.policy).map_err(|e| format!("failed to load policy: {e:?}"))?;
    let service = PolicyServiceImpl::new(Arc::new(engine));

    if let Some(socket_path) = &cli.socket {
        // ── UDS transport ────────────────────────────────────────────────
        tracing::info!(socket = %socket_path.display(), "starting gRPC server on UDS");

        // Remove stale socket file if it exists.
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let uds = tokio::net::UnixListener::bind(socket_path)?;
        let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

        Server::builder()
            .add_service(PolicyServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await?;
    } else {
        // ── TCP transport ────────────────────────────────────────────────
        let addr = cli.listen.parse()?;
        tracing::info!(%addr, "starting gRPC server on TCP");

        Server::builder()
            .add_service(PolicyServiceServer::new(service))
            .serve(addr)
            .await?;
    }

    Ok(())
}
