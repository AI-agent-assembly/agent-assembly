//! Historical audit log replay for dry-run policy simulation.

use std::io::BufRead;
use std::path::Path;

use serde::Deserialize;

use super::error::SimulationError;

/// A single event extracted from an audit log for simulation replay.
///
/// This is a deserialized subset of `aa_core::AuditEntry` — only the fields
/// needed for policy re-evaluation.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationEvent {
    /// The audit event type (e.g. "ToolCallIntercepted", "PolicyViolation").
    pub event_type: String,
    /// The agent identifier that produced this event.
    pub agent_id: String,
    /// Pre-serialized JSON payload from the original audit entry.
    pub payload: String,
}

/// Reads an audit log JSONL file and produces a sequence of `SimulationEvent`s.
#[derive(Debug)]
pub struct HistoricalReplay {
    /// Parsed events from the audit log file.
    events: Vec<SimulationEvent>,
}

impl HistoricalReplay {
    /// Parse an audit log JSONL file into a replay sequence.
    ///
    /// Each line of the file is expected to be a JSON object matching
    /// the `SimulationEvent` schema. Blank lines are skipped.
    pub fn from_file(path: &Path) -> Result<Self, SimulationError> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event: SimulationEvent =
                serde_json::from_str(trimmed).map_err(|e| {
                    SimulationError::AuditParse(format!("line {}: {e}", line_num + 1))
                })?;
            events.push(event);
        }

        Ok(Self { events })
    }

    /// Returns a slice of all parsed simulation events.
    pub fn events(&self) -> &[SimulationEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_jsonl() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            r#"{{"event_type":"ToolCallIntercepted","agent_id":"agent-1","payload":"{{\"name\":\"read_file\"}}" }}"#
        )
        .unwrap();
        writeln!(
            tmp,
            r#"{{"event_type":"PolicyViolation","agent_id":"agent-2","payload":"{{\"name\":\"delete_db\"}}" }}"#
        )
        .unwrap();

        let replay = HistoricalReplay::from_file(tmp.path()).unwrap();
        assert_eq!(replay.events().len(), 2);
        assert_eq!(replay.events()[0].event_type, "ToolCallIntercepted");
        assert_eq!(replay.events()[0].agent_id, "agent-1");
        assert_eq!(replay.events()[1].event_type, "PolicyViolation");
    }

    #[test]
    fn parse_empty_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let replay = HistoricalReplay::from_file(tmp.path()).unwrap();
        assert!(replay.events().is_empty());
    }

    #[test]
    fn skip_blank_lines() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp).unwrap();
        writeln!(
            tmp,
            r#"{{"event_type":"ToolCallIntercepted","agent_id":"a1","payload":"{{}}"}}"#
        )
        .unwrap();
        writeln!(tmp).unwrap();

        let replay = HistoricalReplay::from_file(tmp.path()).unwrap();
        assert_eq!(replay.events().len(), 1);
    }

    #[test]
    fn malformed_line_returns_error() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "not valid json").unwrap();

        let result = HistoricalReplay::from_file(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("line 1"));
    }

    #[test]
    fn missing_file_returns_error() {
        let result = HistoricalReplay::from_file(Path::new("/tmp/nonexistent-audit-log.jsonl"));
        assert!(result.is_err());
    }
}
