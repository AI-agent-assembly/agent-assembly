/// Stable identifier for an agent — UUID v4 encoded as raw bytes.
///
/// The inner `[u8; 16]` is private. Use [`AgentId::from_bytes`] to construct
/// and [`AgentId::as_bytes`] to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentId([u8; 16]);

impl AgentId {
    /// Construct an [`AgentId`] from raw UUID bytes.
    #[inline]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the raw UUID bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Per-execution session identifier — UUID v4 encoded as raw bytes.
///
/// A new [`SessionId`] is generated for each agent execution. It ties together
/// all governance events within a single run.
///
/// The inner `[u8; 16]` is private. Use [`SessionId::from_bytes`] to construct
/// and [`SessionId::as_bytes`] to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionId([u8; 16]);

impl SessionId {
    /// Construct a [`SessionId`] from raw UUID bytes.
    #[inline]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the raw UUID bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BYTES: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[test]
    fn agent_id_round_trip() {
        let id = AgentId::from_bytes(BYTES);
        assert_eq!(id.as_bytes(), &BYTES);
    }

    #[test]
    fn session_id_round_trip() {
        let id = SessionId::from_bytes(BYTES);
        assert_eq!(id.as_bytes(), &BYTES);
    }

    #[test]
    fn agent_id_equality() {
        let a = AgentId::from_bytes(BYTES);
        let b = AgentId::from_bytes(BYTES);
        assert_eq!(a, b);
    }

    #[test]
    fn session_id_equality() {
        let a = SessionId::from_bytes(BYTES);
        let b = SessionId::from_bytes(BYTES);
        assert_eq!(a, b);
    }

    #[test]
    fn agent_id_copy_semantics() {
        let a = AgentId::from_bytes(BYTES);
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn session_id_copy_semantics() {
        let a = SessionId::from_bytes(BYTES);
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn agent_id_and_session_id_are_distinct_types() {
        // Compile-time check: AgentId and SessionId are different types.
        // If they were the same type, the following would be ambiguous or fail.
        let _agent: AgentId = AgentId::from_bytes(BYTES);
        let _session: SessionId = SessionId::from_bytes(BYTES);
    }
}
