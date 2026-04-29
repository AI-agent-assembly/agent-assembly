//! Persistent, append-only audit writer for governance events.
//!
//! [`AuditWriter`] consumes [`AuditEntry`] values from an async mpsc channel
//! and appends each one as a single JSON line to a per-session JSONL file.
//! The hash chain in [`AuditEntry`] provides tamper-evidence; persistence
//! provides durability across process restarts.

use std::io;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use aa_core::AuditEntry;

/// Append-only JSONL audit writer backed by an mpsc channel.
///
/// Created once at server startup, then moved into a background `tokio::spawn`
/// task via [`AuditWriter::run`].
pub struct AuditWriter {
    receiver: mpsc::Receiver<AuditEntry>,
    file: tokio::io::BufWriter<tokio::fs::File>,
    path: PathBuf,
}

impl AuditWriter {
    /// Create a new writer that appends to `<audit_dir>/<agent_id>-<session_id>.jsonl`.
    ///
    /// Creates the `audit_dir` if it does not exist. Opens the target file in
    /// append mode so existing entries are preserved across restarts.
    pub async fn new(
        audit_dir: PathBuf,
        agent_id: &str,
        session_id: &str,
        receiver: mpsc::Receiver<AuditEntry>,
    ) -> io::Result<Self> {
        tokio::fs::create_dir_all(&audit_dir).await?;

        let filename = format!("{agent_id}-{session_id}.jsonl");
        let path = audit_dir.join(filename);

        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        let file = tokio::io::BufWriter::new(file);

        Ok(Self { receiver, file, path })
    }

    /// Serialize one `AuditEntry` as a JSON line and append to the file.
    async fn append(&mut self, entry: &AuditEntry) -> io::Result<()> {
        let json = serde_json::to_string(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.file.write_all(json.as_bytes()).await?;
        self.file.write_all(b"\n").await?;
        self.file.flush().await?;
        Ok(())
    }

    /// Background consumption loop — call via `tokio::spawn(writer.run())`.
    ///
    /// Drains the channel until the sender is dropped (server shutdown).
    /// Individual write failures are logged but do not kill the pipeline.
    pub async fn run(mut self) {
        tracing::info!(path = %self.path.display(), "audit writer started");
        while let Some(entry) = self.receiver.recv().await {
            if let Err(e) = self.append(&entry).await {
                tracing::error!(
                    error = %e,
                    seq = entry.seq(),
                    "audit write failed"
                );
            }
        }
        // Channel closed — sender dropped during shutdown. Flush remaining data.
        if let Err(e) = self.file.flush().await {
            tracing::error!(error = %e, "audit writer final flush failed");
        }
        tracing::info!(path = %self.path.display(), "audit writer stopped");
    }

    /// Verify the hash chain of a JSONL audit file.
    pub async fn verify_chain(_path: &Path) -> Result<VerifyResult, AuditError> {
        todo!("AuditWriter::verify_chain")
    }

    /// Read the `entry_hash` of the last entry in a JSONL file.
    ///
    /// Returns `None` if the file does not exist or is empty.
    /// Skips blank or incomplete trailing lines (standard JSONL recovery).
    pub async fn read_last_hash(path: &Path) -> io::Result<Option<[u8; 32]>> {
        let file = match tokio::fs::File::open(path).await {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();
        let mut last_hash: Option<[u8; 32]> = None;

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(&line) {
                Ok(entry) => last_hash = Some(*entry.entry_hash()),
                Err(_) => {
                    // Incomplete trailing line from a crash — skip it.
                    continue;
                }
            }
        }
        Ok(last_hash)
    }
}

/// Result of a hash-chain verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    /// `true` if every entry's hash matches and the chain links correctly.
    pub is_valid: bool,
    /// Total number of entries checked.
    pub entries_checked: u64,
    /// Index of the first invalid entry, if any.
    pub first_invalid: Option<u64>,
}

/// Errors that can occur during audit operations.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON deserialization error at line {line}: {source}")]
    Deserialize {
        line: u64,
        source: serde_json::Error,
    },
}
