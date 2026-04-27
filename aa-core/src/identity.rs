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
