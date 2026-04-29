//! gRPC server startup — loads policy, builds service, serves over TCP or UDS.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use tonic::transport::Server;

use crate::audit::AuditWriter;
use crate::engine::PolicyEngine;
use crate::registry::AgentRegistry;
use crate::service::{AgentLifecycleServiceImpl, ApprovalServiceImpl, AuditServiceImpl, PolicyServiceImpl};
use aa_core::AuditEntry;
use aa_proto::assembly::agent::v1::agent_lifecycle_service_server::AgentLifecycleServiceServer;
use aa_proto::assembly::approval::v1::approval_service_server::ApprovalServiceServer;
use aa_proto::assembly::audit::v1::audit_service_server::AuditServiceServer;
use aa_proto::assembly::policy::v1::policy_service_server::PolicyServiceServer;
use aa_runtime::approval::ApprovalQueue;

/// Default audit directory relative to the system data directory (`~/.aa/audit`).
fn default_audit_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aa")
        .join("audit")
}

/// Resolve the JSONL path for the given agent/session pair.
fn audit_file_path(audit_dir: &Path, agent_id: &str, session_id: &str) -> PathBuf {
    audit_dir.join(format!("{agent_id}-{session_id}.jsonl"))
}

/// Create the audit channel, spawn the background `AuditWriter`, and return
/// the sender, drop counter, and the last persisted hash (for chain continuity).
async fn setup_audit(
    agent_id: &str,
    session_id: &str,
) -> Result<(tokio::sync::mpsc::Sender<AuditEntry>, Arc<AtomicU64>, [u8; 32]), Box<dyn std::error::Error>> {
    let audit_dir = default_audit_dir();

    // Read the last hash from the existing JSONL file (if any) so the hash
    // chain is maintained across process restarts.
    let audit_path = audit_file_path(&audit_dir, agent_id, session_id);
    let initial_hash = AuditWriter::read_last_hash(&audit_path).await?.unwrap_or([0u8; 32]);

    let (audit_tx, audit_rx) = tokio::sync::mpsc::channel::<AuditEntry>(4096);
    let audit_drops = Arc::new(AtomicU64::new(0));

    let writer = AuditWriter::new(audit_dir, agent_id, session_id, audit_rx).await?;
    tokio::spawn(writer.run());

    Ok((audit_tx, audit_drops, initial_hash))
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
    let (audit_tx, audit_drops, initial_hash) = setup_audit("gateway", "default").await?;
    let policy_svc = PolicyServiceImpl::new(
        Arc::new(engine),
        audit_tx.clone(),
        Arc::clone(&audit_drops),
        initial_hash,
    );
    let audit_svc = AuditServiceImpl::new(audit_tx, audit_drops, initial_hash);
    let lifecycle_svc = AgentLifecycleServiceImpl::new(registry);

    let addr = listen_addr.parse()?;
    tracing::info!(%addr, "starting gRPC server on TCP");

    Server::builder()
        .add_service(PolicyServiceServer::new(policy_svc))
        .add_service(AuditServiceServer::new(audit_svc))
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
    let (audit_tx, audit_drops, initial_hash) = setup_audit("gateway", "default").await?;
    let policy_svc = PolicyServiceImpl::new(
        Arc::new(engine),
        audit_tx.clone(),
        Arc::clone(&audit_drops),
        initial_hash,
    );
    let audit_svc = AuditServiceImpl::new(audit_tx, audit_drops, initial_hash);
    let lifecycle_svc = AgentLifecycleServiceImpl::new(registry);

    tracing::info!(socket = %socket_path.display(), "starting gRPC server on UDS");

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let uds = tokio::net::UnixListener::bind(socket_path)?;
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

    Server::builder()
        .add_service(PolicyServiceServer::new(policy_svc))
        .add_service(AuditServiceServer::new(audit_svc))
        .add_service(AgentLifecycleServiceServer::new(lifecycle_svc))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
