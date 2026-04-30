//! Read-only query interface for JSONL audit log files.
//!
//! [`AuditReader`] scans the audit directory produced by [`super::audit::AuditWriter`],
//! parses JSONL entries, and returns paginated results in reverse chronological order.

use std::io;
use std::path::PathBuf;

use tokio::io::AsyncBufReadExt;

use aa_core::audit::AuditEventType;
use aa_core::{AgentId, AuditEntry};

/// Read-only query interface for the JSONL audit log directory.
///
/// Reads files directly — does not tap the `AuditWriter` mpsc channel.
/// Safe to use concurrently with an active writer because each JSONL line
/// is self-contained and the reader skips incomplete trailing lines.
pub struct AuditReader {
    dir: PathBuf,
}

impl AuditReader {
    /// Create a new reader targeting the given audit directory.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// List audit entries with pagination and optional filters.
    ///
    /// Returns `(entries, total_matching)` where entries are sorted in
    /// reverse chronological order (newest first) and sliced to the
    /// requested `limit`/`offset` window.
    pub async fn list(
        &self,
        limit: usize,
        offset: usize,
        agent_id: Option<&str>,
        event_type: Option<&str>,
    ) -> io::Result<(Vec<AuditEntry>, u64)> {
        let mut all_entries = self.read_all_entries().await?;

        // Parse filter values once.
        let agent_filter: Option<AgentId> = agent_id.and_then(parse_agent_id);
        let event_filter: Option<AuditEventType> = event_type.and_then(parse_event_type);

        // Apply filters.
        if agent_filter.is_some() || event_filter.is_some() {
            all_entries.retain(|entry| {
                if let Some(aid) = &agent_filter {
                    if entry.agent_id() != *aid {
                        return false;
                    }
                }
                if let Some(et) = &event_filter {
                    if entry.event_type() != *et {
                        return false;
                    }
                }
                true
            });
        }

        // Sort by timestamp descending (newest first).
        all_entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp_ns()));

        let total = all_entries.len() as u64;
        let page: Vec<AuditEntry> = all_entries.into_iter().skip(offset).take(limit).collect();

        Ok((page, total))
    }

    /// Read and parse all JSONL files in the audit directory.
    async fn read_all_entries(&self) -> io::Result<Vec<AuditEntry>> {
        let mut entries = Vec::new();

        let mut dir = match tokio::fs::read_dir(&self.dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(entries),
            Err(e) => return Err(e),
        };

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let file = tokio::fs::File::open(&path).await?;
            let reader = tokio::io::BufReader::new(file);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                if line.trim().is_empty() {
                    continue;
                }
                // Skip incomplete or corrupt lines (e.g. partial writes).
                if let Ok(audit_entry) = serde_json::from_str::<AuditEntry>(&line) {
                    entries.push(audit_entry);
                }
            }
        }

        Ok(entries)
    }
}

/// Parse a hex-encoded agent ID string into an [`AgentId`].
fn parse_agent_id(s: &str) -> Option<AgentId> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Some(AgentId::from_bytes(arr))
}

/// Parse an event type string (e.g. `"PolicyViolation"`) into an [`AuditEventType`].
fn parse_event_type(s: &str) -> Option<AuditEventType> {
    match s {
        "ToolCallIntercepted" => Some(AuditEventType::ToolCallIntercepted),
        "PolicyViolation" => Some(AuditEventType::PolicyViolation),
        "CredentialLeakBlocked" => Some(AuditEventType::CredentialLeakBlocked),
        "ApprovalRequested" => Some(AuditEventType::ApprovalRequested),
        "ApprovalGranted" => Some(AuditEventType::ApprovalGranted),
        "ApprovalDenied" => Some(AuditEventType::ApprovalDenied),
        "BudgetLimitApproached" => Some(AuditEventType::BudgetLimitApproached),
        "BudgetLimitExceeded" => Some(AuditEventType::BudgetLimitExceeded),
        "ApprovalTimedOut" => Some(AuditEventType::ApprovalTimedOut),
        _ => None,
    }
}
