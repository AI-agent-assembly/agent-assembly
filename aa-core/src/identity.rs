/// Stable identifier for an agent — UUID v4 encoded as raw bytes.
///
/// The inner `[u8; 16]` is private. Use [`AgentId::from_bytes`] to construct
/// and [`AgentId::as_bytes`] to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
