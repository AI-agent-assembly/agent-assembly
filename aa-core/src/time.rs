//! Timestamp abstraction compatible with `no_std` environments.
//!
//! In `no_std` mode the caller is responsible for supplying the nanosecond
//! value. In `std` mode a [`From<SystemTime>`] convenience impl is available.

/// Nanoseconds since the Unix epoch.
///
/// In `no_std` environments use [`Timestamp::from_nanos`] to construct a value
/// directly. In `std` environments the [`From<std::time::SystemTime>`] impl
/// can be used as a convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construct a [`Timestamp`] from raw nanoseconds since the Unix epoch.
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Return the raw nanosecond value.
    #[inline]
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }
}
