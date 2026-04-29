//! gRPC server startup — loads policy, builds service, serves over TCP or UDS.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use tonic::transport::Server;

use aa_core::AuditEntry;
use crate::audit::AuditWriter;
use crate::engine::PolicyEngine;
use crate::registry::AgentRegistry;
use crate::service::{AgentLifecycleServiceImpl, PolicyServiceImpl};
use aa_proto::assembly::agent::v1::agent_lifecycle_service_server::AgentLifecycleServiceServer;
use aa_proto::assembly::policy::v1::policy_service_server::PolicyServiceServer;

/// Default audit directory relative to the system data directory (`~/.aa/audit`).
fn default_audit_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aa")
        .join("audit")
}

/// Create the audit channel, spawn the background `AuditWriter`, and return
/// the sender + drop counter for injection into services.
async fn setup_audit(
    agent_id: &str,
    session_id: &str,
) -> Result<(tokio::sync::mpsc::Sender<AuditEntry>, Arc<AtomicU64>), Box<dyn std::error::Error>> {
    let audit_dir = default_audit_dir();
    let (audit_tx, audit_rx) = tokio::sync::mpsc::channel::<AuditEntry>(4096);
    let audit_drops = Arc::new(AtomicU64::new(0));

    let writer = AuditWriter::new(audit_dir, agent_id, session_id, audit_rx).await?;
    tokio::spawn(writer.run());

    Ok((audit_tx, audit_drops))
}

/// Start the gRPC server on a TCP address.
///
/// Loads the policy from `policy_path`, wraps it in a `PolicyServiceImpl`, and
/// serves on `listen_addr` (e.g. `"127.0.0.1:50051"`). The `registry` is shared
/// with the `AgentLifecycleService` for agent registration and heartbeat tracking.
pub async fn serve_tcp(
    policy_path: &Path,
    listen_addr: &str,
    registry: Arc<AgentRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let engine = PolicyEngine::load_from_file(policy_path).map_err(|e| format!("failed to load policy: {e:?}"))?;
    let (audit_tx, audit_drops) = setup_audit("gateway", "default").await?;
    let policy_svc = PolicyServiceImpl::new(Arc::new(engine), audit_tx, audit_drops);
    let lifecycle_svc = AgentLifecycleServiceImpl::new(registry);

    let addr = listen_addr.parse()?;
    tracing::info!(%addr, "starting gRPC server on TCP");

    Server::builder()
        .add_service(PolicyServiceServer::new(policy_svc))
        .add_service(AgentLifecycleServiceServer::new(lifecycle_svc))
        .serve(addr)
        .await?;

    Ok(())
}

/// Start the gRPC server on a Unix domain socket.
///
/// Loads the policy from `policy_path`, wraps it in a `PolicyServiceImpl`, and
/// serves on the given `socket_path`. Removes any stale socket file first.
/// The `registry` is shared with the `AgentLifecycleService`.
pub async fn serve_uds(
    policy_path: &Path,
    socket_path: &Path,
    registry: Arc<AgentRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let engine = PolicyEngine::load_from_file(policy_path).map_err(|e| format!("failed to load policy: {e:?}"))?;
    let (audit_tx, audit_drops) = setup_audit("gateway", "default").await?;
    let policy_svc = PolicyServiceImpl::new(Arc::new(engine), audit_tx, audit_drops);
    let lifecycle_svc = AgentLifecycleServiceImpl::new(registry);

    tracing::info!(socket = %socket_path.display(), "starting gRPC server on UDS");

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let uds = tokio::net::UnixListener::bind(socket_path)?;
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

    Server::builder()
        .add_service(PolicyServiceServer::new(policy_svc))
        .add_service(AgentLifecycleServiceServer::new(lifecycle_svc))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
