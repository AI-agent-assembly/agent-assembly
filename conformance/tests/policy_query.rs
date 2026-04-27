//! Conformance tests for policy query round-trip vectors.
//!
//! Each vector describes a CheckActionRequest/CheckActionResponse pair.
//! These tests verify:
//!   1. The request/response JSON deserialises without error
//!   2. The expected decision field matches the declared decision name
//!   3. Decision-specific invariants hold (e.g. approval_id non-empty for PENDING,
//!      redact rules present for REDACT, redact null for ALLOW/DENY)
//!
//! Vectors: `conformance/vectors/policy_query/*.json`

use serde_json::Value;
use std::path::Path;

fn load_policy_vectors() -> Vec<Value> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors/policy_query");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries
        .iter()
        .map(|e| {
            let raw = std::fs::read_to_string(e.path()).unwrap();
            serde_json::from_str(&raw).unwrap_or_else(|err| panic!("parse {}: {err}", e.path().display()))
        })
        .collect()
}

// ── ALLOW decisions ───────────────────────────────────────────────────────────

#[test]
fn allow_decisions_have_no_redact_and_no_approval_id() {
    let vectors = load_policy_vectors();
    let allow_vecs: Vec<_> = vectors
        .iter()
        .filter(|v| v["response"]["decision"] == "ALLOW")
        .collect();

    assert!(!allow_vecs.is_empty(), "no ALLOW vectors found");
    for v in allow_vecs {
        let resp = &v["response"];
        assert!(
            resp["redact"].is_null(),
            "ALLOW response must have null redact: {}",
            v["description"]
        );
        assert_eq!(
            resp["approval_id"].as_str().unwrap_or(""),
            "",
            "ALLOW response must have empty approval_id: {}",
            v["description"]
        );
        assert!(
            resp["decision_latency_us"].as_u64().unwrap_or(0) > 0,
            "ALLOW response must have positive latency: {}",
            v["description"]
        );
    }
}

#[test]
fn allow_vectors_cover_llm_call_tool_call_and_network_call() {
    let vectors = load_policy_vectors();
    let action_types: Vec<_> = vectors
        .iter()
        .filter(|v| v["response"]["decision"] == "ALLOW")
        .map(|v| v["request"]["action_type"].as_str().unwrap_or("").to_string())
        .collect();

    assert!(
        action_types.contains(&"LLM_CALL".to_string()),
        "missing LLM_CALL ALLOW vector"
    );
    assert!(
        action_types.contains(&"TOOL_CALL".to_string()),
        "missing TOOL_CALL ALLOW vector"
    );
    assert!(
        action_types.contains(&"NETWORK_CALL".to_string()),
        "missing NETWORK_CALL ALLOW vector"
    );
}

// ── DENY decisions ────────────────────────────────────────────────────────────

#[test]
fn deny_decisions_have_no_redact_and_no_approval_id() {
    let vectors = load_policy_vectors();
    let deny_vecs: Vec<_> = vectors.iter().filter(|v| v["response"]["decision"] == "DENY").collect();

    assert!(!deny_vecs.is_empty(), "no DENY vectors found");
    for v in deny_vecs {
        let resp = &v["response"];
        assert!(
            resp["redact"].is_null(),
            "DENY response must have null redact: {}",
            v["description"]
        );
        assert_eq!(
            resp["approval_id"].as_str().unwrap_or(""),
            "",
            "DENY response must have empty approval_id: {}",
            v["description"]
        );
        assert!(
            !resp["policy_rule"].as_str().unwrap_or("").is_empty(),
            "DENY response must name the blocking policy rule: {}",
            v["description"]
        );
    }
}

// ── PENDING decisions ─────────────────────────────────────────────────────────

#[test]
fn pending_decisions_have_non_empty_approval_id() {
    let vectors = load_policy_vectors();
    let pending_vecs: Vec<_> = vectors
        .iter()
        .filter(|v| v["response"]["decision"] == "PENDING")
        .collect();

    assert!(!pending_vecs.is_empty(), "no PENDING vectors found");
    for v in pending_vecs {
        let approval_id = v["response"]["approval_id"].as_str().unwrap_or("");
        assert!(
            !approval_id.is_empty(),
            "PENDING response must have non-empty approval_id: {}",
            v["description"]
        );
    }
}

// ── REDACT decisions ──────────────────────────────────────────────────────────

#[test]
fn redact_decisions_have_at_least_one_rule() {
    let vectors = load_policy_vectors();
    let redact_vecs: Vec<_> = vectors
        .iter()
        .filter(|v| v["response"]["decision"] == "REDACT")
        .collect();

    assert!(!redact_vecs.is_empty(), "no REDACT vectors found");
    for v in redact_vecs {
        let rules = v["response"]["redact"]["rules"]
            .as_array()
            .expect("REDACT response must have redact.rules array");
        assert!(
            !rules.is_empty(),
            "REDACT response must have at least one rule: {}",
            v["description"]
        );

        for rule in rules {
            assert!(
                !rule["field_path"].as_str().unwrap_or("").is_empty(),
                "each RedactRule must have a field_path: {}",
                v["description"]
            );
            assert!(
                !rule["replacement"].as_str().unwrap_or("").is_empty(),
                "each RedactRule must have a replacement: {}",
                v["description"]
            );
        }
    }
}

#[test]
fn redact_multi_fields_vector_has_two_rules() {
    let vectors = load_policy_vectors();
    let v = vectors
        .iter()
        .find(|v| v["description"].as_str().unwrap_or("").contains("multiple"))
        .expect("multi-field REDACT vector not found");

    let rules = v["response"]["redact"]["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 2, "multi-field REDACT vector must have exactly 2 rules");
}

// ── All vectors ───────────────────────────────────────────────────────────────

#[test]
fn all_policy_vectors_have_required_fields() {
    let vectors = load_policy_vectors();
    assert!(
        vectors.len() >= 10,
        "expected at least 10 policy query vectors, got {}",
        vectors.len()
    );

    for v in &vectors {
        assert!(
            !v["description"].as_str().unwrap_or("").is_empty(),
            "missing description"
        );
        assert!(v["request"]["agent_id"].is_object(), "missing request.agent_id");
        assert!(
            !v["request"]["action_type"].as_str().unwrap_or("").is_empty(),
            "missing action_type"
        );
        assert!(
            !v["response"]["decision"].as_str().unwrap_or("").is_empty(),
            "missing decision"
        );
        assert!(
            !v["response"]["policy_rule"].as_str().unwrap_or("").is_empty(),
            "missing policy_rule"
        );
    }
}
