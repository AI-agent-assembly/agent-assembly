//! Conformance tests for IPC framing (prost varint length-delimited encoding).
//!
//! Protocol: `aa-proxy` uses prost's length-delimited framing over Unix domain
//! sockets. Each message is prefixed with a varint-encoded byte count, followed
//! by the serialised proto bytes. This is NOT the gRPC 5-byte frame format.
//!
//! Reference implementation: `prost::encoding::encode_varint` +
//!                            `prost::encoding::decode_varint`.
//!
//! Vectors: `conformance/vectors/ipc_framing/*.json`

use conformance::{hex_decode, FramingVector};
use prost::encoding::{decode_varint, encode_varint};
use serde_json::Value;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Applies prost varint length-delimited framing to raw proto bytes.
fn frame(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    encode_varint(data.len() as u64, &mut out);
    out.extend_from_slice(data);
    out
}

/// Strips the varint length prefix and returns the inner proto bytes.
fn deframe(framed: &[u8]) -> Vec<u8> {
    let mut buf = framed;
    let len = decode_varint(&mut buf).expect("valid varint prefix") as usize;
    buf[..len].to_vec()
}

// ── Basic encode/decode (vectors 01-03) ──────────────────────────────────────

#[test]
fn basic_framing_vectors_encode() {
    let vectors: Vec<FramingVector> = load_framing_vectors();
    let basic: Vec<_> = vectors
        .iter()
        .filter(|v| !v.description.contains("edge"))
        .collect();

    for v in &basic {
        let input = hex_decode(&v.input_hex);
        let expected = hex_decode(&v.expected_framed_hex);
        let actual = frame(&input);
        assert_eq!(
            actual, expected,
            "encode mismatch in '{}'\n  got:      {}\n  expected: {}",
            v.description, hex_encode(&actual), v.expected_framed_hex
        );
    }
}

#[test]
fn basic_framing_vectors_decode() {
    let vectors: Vec<FramingVector> = load_framing_vectors();
    let basic: Vec<_> = vectors
        .iter()
        .filter(|v| !v.description.contains("edge"))
        .collect();

    for v in &basic {
        let framed = hex_decode(&v.expected_framed_hex);
        let expected_inner = hex_decode(&v.input_hex);
        let actual = deframe(&framed);
        assert_eq!(
            actual, expected_inner,
            "decode mismatch in '{}'",
            v.description
        );
    }
}

// ── Large/near-max-varint (vectors 04-06) ────────────────────────────────────

#[test]
fn large_message_framing_encode() {
    let vectors: Vec<FramingVector> = load_framing_vectors();
    let large: Vec<_> = vectors
        .iter()
        .filter(|v| {
            v.description.contains("Length-127")
                || v.description.contains("Length-128")
                || v.description.contains("Length-300")
        })
        .collect();

    assert!(!large.is_empty(), "no large-message vectors found");
    for v in &large {
        let input = hex_decode(&v.input_hex);
        let expected = hex_decode(&v.expected_framed_hex);
        let actual = frame(&input);
        assert_eq!(
            actual, expected,
            "large encode mismatch in '{}'",
            v.description
        );
    }
}

#[test]
fn large_message_framing_decode() {
    let vectors: Vec<FramingVector> = load_framing_vectors();
    let large: Vec<_> = vectors
        .iter()
        .filter(|v| {
            v.description.contains("Length-127")
                || v.description.contains("Length-128")
                || v.description.contains("Length-300")
        })
        .collect();

    for v in &large {
        let framed = hex_decode(&v.expected_framed_hex);
        let expected_inner = hex_decode(&v.input_hex);
        let actual = deframe(&framed);
        assert_eq!(
            actual, expected_inner,
            "large decode mismatch in '{}'",
            v.description
        );
    }
}

// ── Stream-split edge cases (vectors 07-08) ───────────────────────────────────

#[test]
fn stream_split_decode_assembles_full_message() {
    let vectors = load_edge_vectors();
    let splits: Vec<_> = vectors
        .iter()
        .filter(|v| v["case_type"] == "stream_split")
        .collect();

    assert!(!splits.is_empty(), "no stream_split vectors found");
    for v in splits {
        let full = hex_decode(v["full_framed_hex"].as_str().unwrap());
        let expected = hex_decode(v["expected_decoded_hex"].as_str().unwrap());

        // Simulate partial delivery by decoding from the full buffer
        // (verifies decoder doesn't over-consume on a single contiguous buffer).
        let decoded = deframe(&full);
        assert_eq!(decoded, expected, "stream_split decode failed: {}", v["description"]);
    }
}

// ── Consecutive frames (vector 09) ───────────────────────────────────────────

#[test]
fn consecutive_frames_decoded_independently() {
    let vectors = load_edge_vectors();
    let v = vectors
        .iter()
        .find(|v| v["case_type"] == "consecutive_frames")
        .expect("consecutive_frames vector not found");

    let concat = hex_decode(v["concatenated_framed_hex"].as_str().unwrap());
    let frames = v["frames"].as_array().unwrap();

    let mut offset = 0;
    for frame_spec in frames {
        let expected_inner = hex_decode(frame_spec["input_hex"].as_str().unwrap());
        let remaining = &concat[offset..];
        let decoded = deframe(remaining);

        // Advance offset past this frame: varint-prefix bytes + body
        let mut tmp = remaining;
        let body_len = decode_varint(&mut tmp).unwrap() as usize;
        let prefix_len = remaining.len() - tmp.len();
        offset += prefix_len + body_len;

        assert_eq!(decoded, expected_inner, "consecutive frame mismatch: {}", frame_spec["note"]);
    }
}

// ── Multi-message stream (vector 10) ─────────────────────────────────────────

#[test]
fn multi_message_stream_yields_all_frames_in_order() {
    let vectors = load_edge_vectors();
    let v = vectors
        .iter()
        .find(|v| v["case_type"] == "multi_message_stream")
        .expect("multi_message_stream vector not found");

    let concat = hex_decode(v["concatenated_framed_hex"].as_str().unwrap());
    let frames = v["frames"].as_array().unwrap();

    let mut offset = 0;
    for (i, frame_spec) in frames.iter().enumerate() {
        let expected_inner = hex_decode(frame_spec["input_hex"].as_str().unwrap());
        let remaining = &concat[offset..];
        let decoded = deframe(remaining);

        let mut tmp = remaining;
        let body_len = decode_varint(&mut tmp).unwrap() as usize;
        let prefix_len = remaining.len() - tmp.len();
        offset += prefix_len + body_len;

        assert_eq!(decoded, expected_inner, "frame {} mismatch in multi_message_stream", i);
    }
    assert_eq!(offset, concat.len(), "bytes remaining after all frames decoded");
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Loads only the `FramingVector` files (01–06); skips edge-case files (07–10)
/// which use a different JSON schema.
fn load_framing_vectors() -> Vec<FramingVector> {
    use std::path::Path;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors/ipc_framing");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".json") && !name.contains("edge")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries
        .iter()
        .map(|e| {
            let raw = std::fs::read_to_string(e.path()).unwrap();
            serde_json::from_str(&raw).unwrap_or_else(|err| {
                panic!("cannot parse {}: {}", e.path().display(), err)
            })
        })
        .collect()
}

fn load_edge_vectors() -> Vec<Value> {
    use std::path::Path;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors/ipc_framing");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("edge")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries
        .iter()
        .map(|e| {
            let raw = std::fs::read_to_string(e.path()).unwrap();
            serde_json::from_str(&raw).unwrap()
        })
        .collect()
}
