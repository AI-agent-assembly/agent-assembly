//! Token bucket rate limiter for per-tool call limits.

use std::time::Instant;

/// Token bucket rate limiter.
///
/// Implements a token bucket algorithm that refills at a rate of `capacity` tokens per hour.
/// Used to enforce per-tool call limits.
#[allow(dead_code)]
pub(crate) struct TokenBucket {
    capacity: u32,
    tokens: f64,
    pub(crate) last_refill: Instant,
}

impl TokenBucket {
    #[allow(dead_code)]
    /// Create a new token bucket with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of tokens and refill rate per hour
    pub(crate) fn new(capacity: u32) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            last_refill: Instant::now(),
        }
    }

    #[allow(dead_code)]
    /// Try to consume one token from the bucket.
    ///
    /// Refills tokens based on elapsed time since last call, then attempts to consume one token.
    /// Tokens refill at a rate of `capacity` tokens per hour (3600 seconds).
    ///
    /// Returns `true` if a token was consumed, `false` if the bucket is empty.
    pub(crate) fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_secs = (now - self.last_refill).as_secs_f64();
        self.tokens = f64::min(
            self.capacity as f64,
            self.tokens + self.capacity as f64 * elapsed_secs / 3600.0,
        );
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn new_bucket_allows_up_to_capacity() {
        let mut bucket = TokenBucket::new(3);
        // Should be able to consume up to capacity
        assert!(bucket.try_consume(), "First consume should succeed");
        assert!(bucket.try_consume(), "Second consume should succeed");
        assert!(bucket.try_consume(), "Third consume should succeed");
        // Fourth consume should fail (no tokens left)
        assert!(!bucket.try_consume(), "Fourth consume should fail (capacity exceeded)");
    }

    #[test]
    fn bucket_refills_proportionally_after_half_hour() {
        let mut bucket = TokenBucket::new(60);
        // Consume all tokens
        for _ in 0..60 {
            bucket.try_consume();
        }
        // Verify bucket is empty
        assert!(
            !bucket.try_consume(),
            "Bucket should be empty after consuming all tokens"
        );

        // Manually set last_refill to 30 minutes ago (1800 seconds)
        bucket.last_refill = Instant::now() - Duration::from_secs(1800);

        // Try to consume - should succeed because 30 tokens should have refilled
        assert!(
            bucket.try_consume(),
            "Should be able to consume after refill (30 tokens refilled in 30 min)"
        );
    }

    #[test]
    fn bucket_does_not_exceed_capacity_on_refill() {
        let mut bucket = TokenBucket::new(10);
        // Manually set last_refill to 2 hours ago (7200 seconds)
        // This would normally refill 20 tokens, but should be capped at capacity (10)
        bucket.last_refill = Instant::now() - Duration::from_secs(7200);

        // Consume the capped 10 tokens
        for _ in 0..10 {
            assert!(bucket.try_consume(), "Should consume token");
        }

        // 11th consume should fail
        assert!(!bucket.try_consume(), "11th consume should fail (capacity is 10)");
    }
}
