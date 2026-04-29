//! Unit tests for `AgentRegistry` CRUD operations and control stream infrastructure.

use std::collections::BTreeMap;

use chrono::Utc;

use aa_gateway::registry::store::AgentRecord;
use aa_gateway::registry::{AgentRegistry, AgentStatus};

/// Build a minimal `AgentRecord` with the given 16-byte key.
fn make_record(key: [u8; 16]) -> AgentRecord {
    AgentRecord {
        agent_id: key,
        name: "test-agent".into(),
        framework: "custom".into(),
        version: "0.1.0".into(),
        risk_tier: 0,
        tool_names: vec!["tool_a".into()],
        public_key: "pk_placeholder".into(),
        credential_token: "tok_placeholder".into(),
        metadata: BTreeMap::new(),
        registered_at: Utc::now(),
        last_heartbeat: Utc::now(),
        status: AgentStatus::Active,
    }
}

fn key(n: u8) -> [u8; 16] {
    let mut k = [0u8; 16];
    k[0] = n;
    k
}

// ── Register ────────────────────────────────────────────────────────────────

#[test]
fn register_inserts_agent() {
    let reg = AgentRegistry::new();
    let record = make_record(key(1));
    reg.register(record).unwrap();

    let got = reg.get(&key(1)).expect("agent should exist");
    assert_eq!(got.name, "test-agent");
    assert_eq!(got.framework, "custom");
}

#[test]
fn register_duplicate_returns_error() {
    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();

    let err = reg.register(make_record(key(1)));
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("already registered"));
}

// ── Get ─────────────────────────────────────────────────────────────────────

#[test]
fn get_returns_none_for_missing_agent() {
    let reg = AgentRegistry::new();
    assert!(reg.get(&key(99)).is_none());
}

// ── Deregister ──────────────────────────────────────────────────────────────

#[test]
fn deregister_removes_agent() {
    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();

    let removed = reg.deregister(&key(1)).unwrap();
    assert_eq!(removed.name, "test-agent");
    assert!(reg.get(&key(1)).is_none());
}

#[test]
fn deregister_missing_returns_error() {
    let reg = AgentRegistry::new();
    let err = reg.deregister(&key(1));
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("not found"));
}

// ── Heartbeat ───────────────────────────────────────────────────────────────

#[test]
fn update_heartbeat_updates_timestamp() {
    let reg = AgentRegistry::new();
    let mut record = make_record(key(1));
    let old_ts = Utc::now() - chrono::Duration::hours(1);
    record.last_heartbeat = old_ts;
    reg.register(record).unwrap();

    reg.update_heartbeat(&key(1)).unwrap();

    let got = reg.get(&key(1)).unwrap();
    assert!(got.last_heartbeat > old_ts);
}

#[test]
fn update_heartbeat_missing_returns_error() {
    let reg = AgentRegistry::new();
    assert!(reg.update_heartbeat(&key(99)).is_err());
}

// ── List ────────────────────────────────────────────────────────────────────

#[test]
fn list_returns_all_agents() {
    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();
    reg.register(make_record(key(2))).unwrap();
    reg.register(make_record(key(3))).unwrap();

    let agents = reg.list();
    assert_eq!(agents.len(), 3);
}

#[test]
fn list_empty_registry() {
    let reg = AgentRegistry::new();
    assert!(reg.list().is_empty());
}

// ── Control stream ──────────────────────────────────────────────────────────

#[tokio::test]
async fn open_control_stream_for_registered_agent() {
    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();

    let _rx = reg.open_control_stream(&key(1)).expect("should open stream");
}

#[test]
fn open_control_stream_for_missing_agent_returns_error() {
    let reg = AgentRegistry::new();
    assert!(reg.open_control_stream(&key(99)).is_err());
}

#[tokio::test]
async fn send_command_delivers_to_stream() {
    use aa_proto::assembly::agent::v1::control_command::Command;
    use aa_proto::assembly::agent::v1::{ControlCommand, SuspendCommand};

    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();
    let mut rx = reg.open_control_stream(&key(1)).unwrap();

    let cmd = ControlCommand {
        command: Some(Command::Suspend(SuspendCommand {
            reason: "test suspend".into(),
        })),
    };
    reg.send_command(&key(1), cmd).await.unwrap();

    let received = rx.recv().await.unwrap().unwrap();
    match received.command {
        Some(Command::Suspend(s)) => assert_eq!(s.reason, "test suspend"),
        other => panic!("expected Suspend command, got {other:?}"),
    }
}

#[tokio::test]
async fn deregister_cleans_up_control_sender() {
    use aa_proto::assembly::agent::v1::control_command::Command;
    use aa_proto::assembly::agent::v1::{ControlCommand, SuspendCommand};

    let reg = AgentRegistry::new();
    reg.register(make_record(key(1))).unwrap();
    let _rx = reg.open_control_stream(&key(1)).unwrap();

    reg.deregister(&key(1)).unwrap();

    // send_command should fail since sender was removed
    let cmd = ControlCommand {
        command: Some(Command::Suspend(SuspendCommand { reason: "noop".into() })),
    };
    assert!(reg.send_command(&key(1), cmd).await.is_err());
}
