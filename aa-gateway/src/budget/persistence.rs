//! Budget persistence — STUB (will be fully implemented in Tasks 25–32).

use crate::budget::types::BudgetState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedAgentEntry {
    pub agent_id_hex: String,
    pub state: BudgetState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedBudget {
    pub per_agent: Vec<PersistedAgentEntry>,
    pub global: BudgetState,
}

pub fn agent_id_to_hex(id: &aa_core::AgentId) -> String {
    id.as_bytes().iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hex_to_agent_id(hex: &str) -> Result<aa_core::AgentId, std::io::Error> {
    if hex.len() != 32 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad hex len"));
    }
    let mut bytes = [0u8; 16];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0]).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let lo = hex_nibble(chunk[1]).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        bytes[i] = (hi << 4) | lo;
    }
    Ok(aa_core::AgentId::from_bytes(bytes))
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex byte: {b}")),
    }
}
