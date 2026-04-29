//! gRPC server startup — loads policy, builds service, serves over TCP or UDS.

use std::path::Path;
use std::sync::Arc;

use tonic::transport::Server;

use crate::engine::PolicyEngine;
use crate::service::PolicyServiceImpl;
use aa_proto::assembly::policy::v1::policy_service_server::PolicyServiceServer;

/// Start the gRPC server on a TCP address.
///
/// Loads the policy from `policy_path`, wraps it in a `PolicyServiceImpl`, and
/// serves on `listen_addr` (e.g. `"127.0.0.1:50051"`).
pub async fn serve_tcp(policy_path: &Path, listen_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let engine = PolicyEngine::load_from_file(policy_path).map_err(|e| format!("failed to load policy: {e:?}"))?;
    let service = PolicyServiceImpl::new(Arc::new(engine));

    let addr = listen_addr.parse()?;
    tracing::info!(%addr, "starting gRPC server on TCP");

    Server::builder()
        .add_service(PolicyServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

/// Start the gRPC server on a Unix domain socket.
///
/// Loads the policy from `policy_path`, wraps it in a `PolicyServiceImpl`, and
/// serves on the given `socket_path`. Removes any stale socket file first.
pub async fn serve_uds(policy_path: &Path, socket_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let engine = PolicyEngine::load_from_file(policy_path).map_err(|e| format!("failed to load policy: {e:?}"))?;
    let service = PolicyServiceImpl::new(Arc::new(engine));

    tracing::info!(socket = %socket_path.display(), "starting gRPC server on UDS");

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let uds = tokio::net::UnixListener::bind(socket_path)?;
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

    Server::builder()
        .add_service(PolicyServiceServer::new(service))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
