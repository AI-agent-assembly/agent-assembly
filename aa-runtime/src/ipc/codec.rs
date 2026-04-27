//! Length-framed binary codec for the IPC socket protocol.
//!
//! Wire format: `[1-byte tag][prost varint length][prost-encoded payload]`
//!
//! Inbound tags (SDK → runtime):
//!   1 = PolicyQuery  (CheckActionRequest)
//!   2 = EventReport  (AuditEvent)
//!   3 = ApprovalResponse (ApprovalDecision)
//!   4 = Heartbeat    (no payload)
//!
//! Outbound tags (runtime → SDK):
//!   1 = PolicyResponse   (CheckActionResponse)
//!   2 = ApprovalDecision (ApprovalDecision)
//!   3 = Ack              (no payload)

use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ipc::message::{IpcFrame, IpcResponse};
use aa_proto::assembly::audit::v1::AuditEvent;
use aa_proto::assembly::event::v1::ApprovalDecision;
use aa_proto::assembly::policy::v1::CheckActionRequest;
#[cfg(test)]
use aa_proto::assembly::policy::v1::CheckActionResponse;

// ── Inbound tag constants ─────────────────────────────────────────────────────

pub const TAG_POLICY_QUERY: u8 = 1;
pub const TAG_EVENT_REPORT: u8 = 2;
pub const TAG_APPROVAL_RESPONSE: u8 = 3;
pub const TAG_HEARTBEAT: u8 = 4;

// ── Outbound tag constants ────────────────────────────────────────────────────

pub const TAG_POLICY_RESPONSE: u8 = 1;
pub const TAG_APPROVAL_DECISION: u8 = 2;
pub const TAG_ACK: u8 = 3;

/// Errors that can occur during frame encoding or decoding.
#[derive(Debug)]
pub enum CodecError {
    Io(std::io::Error),
    UnknownTag(u8),
    DecodeError(prost::DecodeError),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::Io(e) => write!(f, "IO error: {e}"),
            CodecError::UnknownTag(t) => write!(f, "unknown frame tag: {t}"),
            CodecError::DecodeError(e) => write!(f, "prost decode error: {e}"),
        }
    }
}

impl From<std::io::Error> for CodecError {
    fn from(e: std::io::Error) -> Self {
        CodecError::Io(e)
    }
}

impl From<prost::DecodeError> for CodecError {
    fn from(e: prost::DecodeError) -> Self {
        CodecError::DecodeError(e)
    }
}

/// Read one `IpcFrame` from an async reader.
///
/// Reads a 1-byte tag, then a prost length-delimited payload, and returns
/// the decoded `IpcFrame`.
pub async fn read_frame<R>(reader: &mut R) -> Result<IpcFrame, CodecError>
where
    R: AsyncReadExt + Unpin,
{
    // Read the 1-byte tag.
    let tag = reader.read_u8().await?;

    match tag {
        TAG_HEARTBEAT => Ok(IpcFrame::Heartbeat),
        TAG_POLICY_QUERY => {
            let bytes = read_length_delimited(reader).await?;
            let msg = CheckActionRequest::decode(bytes.as_ref())?;
            Ok(IpcFrame::PolicyQuery(msg))
        }
        TAG_EVENT_REPORT => {
            let bytes = read_length_delimited(reader).await?;
            let msg = AuditEvent::decode(bytes.as_ref())?;
            Ok(IpcFrame::EventReport(msg))
        }
        TAG_APPROVAL_RESPONSE => {
            let bytes = read_length_delimited(reader).await?;
            let msg = ApprovalDecision::decode(bytes.as_ref())?;
            Ok(IpcFrame::ApprovalResponse(msg))
        }
        other => Err(CodecError::UnknownTag(other)),
    }
}

/// Write one `IpcResponse` to an async writer.
pub async fn write_response<W>(writer: &mut W, response: IpcResponse) -> Result<(), CodecError>
where
    W: AsyncWriteExt + Unpin,
{
    match response {
        IpcResponse::Ack => {
            writer.write_u8(TAG_ACK).await?;
            write_length_delimited(writer, &[]).await?;
        }
        IpcResponse::PolicyResponse(msg) => {
            writer.write_u8(TAG_POLICY_RESPONSE).await?;
            let bytes = msg.encode_to_vec();
            write_length_delimited(writer, &bytes).await?;
        }
        IpcResponse::ApprovalDecision(msg) => {
            writer.write_u8(TAG_APPROVAL_DECISION).await?;
            let bytes = msg.encode_to_vec();
            write_length_delimited(writer, &bytes).await?;
        }
    }
    writer.flush().await?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read a prost-style length-delimited payload: varint length then `length` bytes.
async fn read_length_delimited<R>(reader: &mut R) -> Result<Vec<u8>, CodecError>
where
    R: AsyncReadExt + Unpin,
{
    // Read the varint length (prost uses unsigned varint).
    let len = read_varint(reader).await? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write a prost-style length-delimited payload: varint length then bytes.
async fn write_length_delimited<W>(writer: &mut W, bytes: &[u8]) -> Result<(), CodecError>
where
    W: AsyncWriteExt + Unpin,
{
    write_varint(writer, bytes.len() as u64).await?;
    writer.write_all(bytes).await?;
    Ok(())
}

/// Read a protobuf-style unsigned varint from an async reader.
async fn read_varint<R>(reader: &mut R) -> Result<u64, CodecError>
where
    R: AsyncReadExt + Unpin,
{
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        let byte = reader.read_u8().await?;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(CodecError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "varint too long",
            )));
        }
    }
    Ok(result)
}

/// Write a protobuf-style unsigned varint to an async writer.
async fn write_varint<W>(writer: &mut W, mut value: u64) -> Result<(), CodecError>
where
    W: AsyncWriteExt + Unpin,
{
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            writer.write_u8(byte).await?;
            break;
        } else {
            writer.write_u8(byte | 0x80).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_proto::assembly::audit::v1::AuditEvent;
    use aa_proto::assembly::event::v1::ApprovalDecision;
    use aa_proto::assembly::policy::v1::{CheckActionRequest, CheckActionResponse};
    use std::io::Cursor;

    // Helper: encode a response to bytes using a Vec writer
    async fn encode_response(response: IpcResponse) -> Vec<u8> {
        let mut buf = Vec::new();
        write_response(&mut buf, response).await.unwrap();
        buf
    }

    #[tokio::test]
    async fn heartbeat_round_trip() {
        // Write a Heartbeat frame
        let mut buf: Vec<u8> = Vec::new();
        buf.push(TAG_HEARTBEAT);
        write_varint(&mut buf, 0).await.unwrap();

        let mut cursor = Cursor::new(buf);
        let frame = read_frame(&mut cursor).await.unwrap();

        assert!(matches!(frame, IpcFrame::Heartbeat));
    }

    #[tokio::test]
    async fn ack_response_encodes_and_has_correct_tag() {
        let bytes = encode_response(IpcResponse::Ack).await;
        assert_eq!(bytes[0], TAG_ACK);
    }

    #[tokio::test]
    async fn policy_query_round_trip() {
        let request = CheckActionRequest {
            trace_id: "trace-abc".to_string(),
            ..Default::default()
        };

        // Encode as inbound frame manually
        let mut buf: Vec<u8> = Vec::new();
        buf.push(TAG_POLICY_QUERY);
        let payload = request.encode_to_vec();
        write_varint(&mut buf, payload.len() as u64).await.unwrap();
        buf.extend_from_slice(&payload);

        // Decode
        let mut cursor = Cursor::new(buf);
        let frame = read_frame(&mut cursor).await.unwrap();

        match frame {
            IpcFrame::PolicyQuery(decoded) => {
                assert_eq!(decoded.trace_id, "trace-abc");
            }
            other => panic!("expected PolicyQuery, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn event_report_round_trip() {
        let event = AuditEvent {
            event_id: "evt-123".to_string(),
            ..Default::default()
        };

        let mut buf: Vec<u8> = Vec::new();
        buf.push(TAG_EVENT_REPORT);
        let payload = event.encode_to_vec();
        write_varint(&mut buf, payload.len() as u64).await.unwrap();
        buf.extend_from_slice(&payload);

        let mut cursor = Cursor::new(buf);
        let frame = read_frame(&mut cursor).await.unwrap();

        match frame {
            IpcFrame::EventReport(decoded) => {
                assert_eq!(decoded.event_id, "evt-123");
            }
            other => panic!("expected EventReport, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn approval_response_round_trip() {
        let decision = ApprovalDecision {
            approval_id: "appr-999".to_string(),
            approved: true,
            decided_by: "reviewer-1".to_string(),
            ..Default::default()
        };

        let mut buf: Vec<u8> = Vec::new();
        buf.push(TAG_APPROVAL_RESPONSE);
        let payload = decision.encode_to_vec();
        write_varint(&mut buf, payload.len() as u64).await.unwrap();
        buf.extend_from_slice(&payload);

        let mut cursor = Cursor::new(buf);
        let frame = read_frame(&mut cursor).await.unwrap();

        match frame {
            IpcFrame::ApprovalResponse(decoded) => {
                assert_eq!(decoded.approval_id, "appr-999");
                assert!(decoded.approved);
            }
            other => panic!("expected ApprovalResponse, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn policy_response_encodes_correctly() {
        let response = CheckActionResponse {
            reason: "allowed by policy".to_string(),
            ..Default::default()
        };

        let bytes = encode_response(IpcResponse::PolicyResponse(response)).await;

        assert_eq!(bytes[0], TAG_POLICY_RESPONSE);
        // Decode back: skip tag byte, read varint length, then decode payload.
        // The varint length sits between the tag byte and the payload.
        // Re-parse the length from the buffer to find where the payload starts.
        let mut cursor = Cursor::new(&bytes[1..]);
        let len = read_varint(&mut cursor).await.unwrap() as usize;
        let payload_start = bytes.len() - len;
        let payload = &bytes[payload_start..];
        let decoded = CheckActionResponse::decode(payload).unwrap();
        assert_eq!(decoded.reason, "allowed by policy");
    }

    #[tokio::test]
    async fn unknown_tag_returns_error() {
        let buf = vec![99u8, 0u8]; // tag=99, length=0
        let mut cursor = Cursor::new(buf);
        let result = read_frame(&mut cursor).await;
        assert!(matches!(result, Err(CodecError::UnknownTag(99))));
    }
}
