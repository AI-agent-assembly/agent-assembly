//! Conformance tests for the agent session lifecycle protocol.
//!
//! Each JSON vector in `vectors/session_lifecycle/` specifies the field values
//! for one lifecycle message. Tests load the vectors, construct the corresponding
//! aa-proto message, round-trip it through prost encode/decode, and verify that
//! key fields survive the wire-format serialisation.

use aa_proto::assembly::agent::v1::{
    control_command::Command, ControlCommand, DeregisterRequest, DeregisterResponse,
    HeartbeatRequest, HeartbeatResponse, KillCommand, PolicyUpdateCommand, RegisterRequest,
    RegisterResponse, ResumeCommand, SuspendCommand,
};
use aa_proto::assembly::common::v1::AgentId;
use conformance::{load_vectors, SessionLifecycleVector};
use prost::Message;

// ── helpers ──────────────────────────────────────────────────────────────────

fn str_field<'a>(fields: &'a serde_json::Value, key: &str) -> &'a str {
    fields[key].as_str().unwrap_or_else(|| panic!("missing string field '{key}' in vector"))
}

fn bool_field(fields: &serde_json::Value, key: &str) -> bool {
    fields[key].as_bool().unwrap_or_else(|| panic!("missing bool field '{key}' in vector"))
}

fn i64_field(fields: &serde_json::Value, key: &str) -> i64 {
    fields[key].as_i64().unwrap_or_else(|| panic!("missing i64 field '{key}' in vector"))
}

fn agent_id_field(fields: &serde_json::Value) -> AgentId {
    let aid = &fields["agent_id"];
    AgentId {
        org_id: aid["org_id"].as_str().unwrap_or("").to_string(),
        team_id: aid["team_id"].as_str().unwrap_or("").to_string(),
        agent_id: aid["agent_id"].as_str().unwrap_or("").to_string(),
    }
}

/// Round-trip `msg` through prost encode→decode and return the decoded bytes length.
fn round_trip<M: Message + Default>(msg: &M) -> M {
    let bytes = msg.encode_to_vec();
    assert!(!bytes.is_empty(), "encoded message must not be empty");
    M::decode(bytes.as_slice()).expect("prost decode must succeed after encode")
}

// ── tests ─────────────────────────────────────────────────────────────────────

fn vectors_of_type(message_type: &str) -> Vec<SessionLifecycleVector> {
    let all: Vec<SessionLifecycleVector> = load_vectors("vectors/session_lifecycle");
    all.into_iter().filter(|v| v.message_type == message_type).collect()
}

#[test]
fn register_request_round_trips() {
    for v in vectors_of_type("RegisterRequest") {
        let f = &v.fields;
        let msg = RegisterRequest {
            agent_id: Some(agent_id_field(f)),
            name: str_field(f, "name").to_string(),
            framework: str_field(f, "framework").to_string(),
            version: str_field(f, "version").to_string(),
            tool_names: f["tool_names"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x.as_str()).map(String::from).collect())
                .unwrap_or_default(),
            public_key: str_field(f, "public_key").to_string(),
            ..Default::default()
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.name, msg.name, "vector '{}': name survives round-trip", v.description);
        assert_eq!(decoded.framework, msg.framework, "vector '{}': framework survives round-trip", v.description);
        assert_eq!(decoded.tool_names, msg.tool_names, "vector '{}': tool_names survive round-trip", v.description);
    }
}

#[test]
fn register_response_round_trips() {
    for v in vectors_of_type("RegisterResponse") {
        let f = &v.fields;
        let msg = RegisterResponse {
            credential_token: str_field(f, "credential_token").to_string(),
            assigned_policy: str_field(f, "assigned_policy").to_string(),
            heartbeat_interval_sec: i64_field(f, "heartbeat_interval_sec"),
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.credential_token, msg.credential_token, "vector '{}': credential_token survives round-trip", v.description);
        assert_eq!(decoded.heartbeat_interval_sec, msg.heartbeat_interval_sec, "vector '{}': heartbeat_interval_sec survives round-trip", v.description);
        assert!(!decoded.credential_token.is_empty(), "vector '{}': credential_token must be non-empty", v.description);
    }
}

#[test]
fn heartbeat_request_round_trips() {
    for v in vectors_of_type("HeartbeatRequest") {
        let f = &v.fields;
        let msg = HeartbeatRequest {
            agent_id: Some(agent_id_field(f)),
            credential_token: str_field(f, "credential_token").to_string(),
            active_runs: f["active_runs"].as_i64().unwrap_or(0) as i32,
            actions_count: i64_field(f, "actions_count"),
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.actions_count, msg.actions_count, "vector '{}': actions_count survives round-trip", v.description);
        assert_eq!(decoded.active_runs, msg.active_runs, "vector '{}': active_runs survives round-trip", v.description);
    }
}

#[test]
fn heartbeat_response_round_trips() {
    for v in vectors_of_type("HeartbeatResponse") {
        let f = &v.fields;
        let msg = HeartbeatResponse {
            policy_updated: bool_field(f, "policy_updated"),
            should_suspend: bool_field(f, "should_suspend"),
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.policy_updated, msg.policy_updated, "vector '{}': policy_updated survives round-trip", v.description);
        assert_eq!(decoded.should_suspend, msg.should_suspend, "vector '{}': should_suspend survives round-trip", v.description);
    }
}

#[test]
fn deregister_request_round_trips() {
    for v in vectors_of_type("DeregisterRequest") {
        let f = &v.fields;
        let msg = DeregisterRequest {
            agent_id: Some(agent_id_field(f)),
            credential_token: str_field(f, "credential_token").to_string(),
            reason: str_field(f, "reason").to_string(),
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.reason, msg.reason, "vector '{}': reason survives round-trip", v.description);
    }
}

#[test]
fn deregister_response_round_trips() {
    for v in vectors_of_type("DeregisterResponse") {
        let f = &v.fields;
        let msg = DeregisterResponse {
            success: bool_field(f, "success"),
            agent_id: str_field(f, "agent_id").to_string(),
        };
        let decoded = round_trip(&msg);
        assert_eq!(decoded.success, msg.success, "vector '{}': success survives round-trip", v.description);
        assert_eq!(decoded.agent_id, msg.agent_id, "vector '{}': agent_id survives round-trip", v.description);
    }
}

#[test]
fn control_suspend_round_trips() {
    for v in vectors_of_type("ControlCommand_Suspend") {
        let f = &v.fields;
        let msg = ControlCommand {
            command: Some(Command::Suspend(SuspendCommand {
                reason: str_field(f, "reason").to_string(),
            })),
        };
        let decoded = round_trip(&msg);
        let Command::Suspend(inner) = decoded.command.expect("command must be Some") else {
            panic!("vector '{}': expected Suspend variant", v.description);
        };
        assert!(!inner.reason.is_empty(), "vector '{}': suspend reason must be non-empty", v.description);
    }
}

#[test]
fn control_resume_round_trips() {
    for v in vectors_of_type("ControlCommand_Resume") {
        let f = &v.fields;
        let msg = ControlCommand {
            command: Some(Command::Resume(ResumeCommand {
                note: str_field(f, "note").to_string(),
            })),
        };
        let decoded = round_trip(&msg);
        let Command::Resume(inner) = decoded.command.expect("command must be Some") else {
            panic!("vector '{}': expected Resume variant", v.description);
        };
        assert!(!inner.note.is_empty(), "vector '{}': resume note must be non-empty", v.description);
    }
}

#[test]
fn control_policy_update_round_trips() {
    for v in vectors_of_type("ControlCommand_PolicyUpdate") {
        let f = &v.fields;
        let msg = ControlCommand {
            command: Some(Command::PolicyUpdate(PolicyUpdateCommand {
                new_policy_id: str_field(f, "new_policy_id").to_string(),
                policy_bytes: vec![],
            })),
        };
        let decoded = round_trip(&msg);
        let Command::PolicyUpdate(inner) = decoded.command.expect("command must be Some") else {
            panic!("vector '{}': expected PolicyUpdate variant", v.description);
        };
        assert!(!inner.new_policy_id.is_empty(), "vector '{}': new_policy_id must be non-empty", v.description);
    }
}

#[test]
fn control_kill_round_trips() {
    for v in vectors_of_type("ControlCommand_Kill") {
        let f = &v.fields;
        let msg = ControlCommand {
            command: Some(Command::Kill(KillCommand {
                reason: str_field(f, "reason").to_string(),
            })),
        };
        let decoded = round_trip(&msg);
        let Command::Kill(inner) = decoded.command.expect("command must be Some") else {
            panic!("vector '{}': expected Kill variant", v.description);
        };
        assert!(!inner.reason.is_empty(), "vector '{}': kill reason must be non-empty", v.description);
    }
}
