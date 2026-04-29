//! Proto ↔ registry type conversions for the AgentLifecycleService.

use aa_proto::assembly::common::v1::AgentId as ProtoAgentId;
use sha2::{Digest, Sha256};

/// Derive a deterministic 16-byte registry key from a composite proto [`AgentId`](ProtoAgentId).
///
/// Hashes `"{org_id}/{team_id}/{agent_id}"` with SHA-256, then truncates to 16 bytes.
pub fn proto_agent_id_to_key(id: &ProtoAgentId) -> [u8; 16] {
    let composite = format!("{}/{}/{}", id.org_id, id.team_id, id.agent_id);
    let digest = Sha256::digest(composite.as_bytes());
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

/// Validate that a proto [`AgentId`](ProtoAgentId) has all required fields populated.
pub fn validate_proto_agent_id(id: &ProtoAgentId) -> Result<(), &'static str> {
    if id.agent_id.is_empty() {
        return Err("agent_id is empty");
    }
    Ok(())
}
