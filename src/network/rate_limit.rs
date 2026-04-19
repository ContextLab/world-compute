//! Token-bucket rate limiting per FR-013.

use crate::error::{ErrorCode, WcError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Rate limit classes with their associated per-minute limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitClass {
    /// Heartbeat messages from donors: 120 per minute.
    DonorHeartbeat,
    /// Job submission requests: 10 per minute.
    JobSubmit,
    /// Governance vote submissions: 5 per minute.
    Governance,
    /// Cluster status queries: 30 per minute.
    ClusterStatus,
    /// Administrative actions: 1 per minute.
    AdminAction,
}

impl RateLimitClass {
    /// Maximum requests allowed per minute for this class.
    pub fn per_minute(self) -> u32 {
        match self {
            Self::DonorHeartbeat => 120,
            Self::JobSubmit => 10,
            Self::Governance => 5,
            Self::ClusterStatus => 30,
            Self::AdminAction => 1,
        }
    }

    /// Tokens per second refill rate.
    pub fn refill_rate(self) -> f64 {
        self.per_minute() as f64 / 60.0
    }
}

/// Per-class token bucket state.
#[derive(Debug)]
pub struct TokenBucket {
    /// Current number of tokens available.
    pub tokens: f64,
    /// Maximum number of tokens (bucket capacity).
    pub max_tokens: f64,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Last time the bucket was refilled.
    pub last_refill: Instant,
}

impl TokenBucket {
    /// Create a new bucket for the given rate limit class.
    pub fn new(per_minute: u32) -> Self {
        let capacity = per_minute as f64;
        Self {
            tokens: capacity,
            max_tokens: capacity,
            refill_rate: capacity / 60.0,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time, then attempt to consume one.
    /// Returns `Ok(())` on success, or `Err` with retry-after seconds on failure.
    pub fn try_consume(&mut self) -> Result<(), f64> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate how long until 1 token is available
            let deficit = 1.0 - self.tokens;
            let retry_after = deficit / self.refill_rate;
            Err(retry_after)
        }
    }
}

/// Error returned when a request is rate-limited.
#[derive(Debug, Clone)]
pub struct RateLimitError {
    /// How many seconds until the caller should retry.
    pub retry_after_secs: f64,
    /// Human-readable message.
    pub message: String,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (retry after {:.1}s)", self.message, self.retry_after_secs)
    }
}

impl std::error::Error for RateLimitError {}

/// Token-bucket rate limiter keyed by `(caller_id, RateLimitClass)`.
///
/// A single `RateLimiter` instance is shared across the process. Each unique
/// (caller, class) pair has an independent bucket.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<(String, RateLimitClass), TokenBucket>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self { buckets: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Check whether `caller_id` is within the rate limit for `class`.
    ///
    /// Returns `Ok(())` if the request is allowed, or a `RateLimited` error
    /// if the bucket is empty.
    pub fn check(&self, caller_id: &str, class: RateLimitClass) -> Result<(), WcError> {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry((caller_id.to_string(), class))
            .or_insert_with(|| TokenBucket::new(class.per_minute()));

        match bucket.try_consume() {
            Ok(()) => Ok(()),
            Err(_retry_after) => Err(WcError::new(
                ErrorCode::RateLimited,
                format!(
                    "Rate limit exceeded for class {class:?}: max {} req/min",
                    class.per_minute()
                ),
            )),
        }
    }

    /// Attempt to acquire a token for the given class and key.
    /// Returns `Ok(())` on success, or `RateLimitError` with `retry_after_secs`.
    pub fn try_acquire(&self, class: RateLimitClass, key: &str) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry((key.to_string(), class))
            .or_insert_with(|| TokenBucket::new(class.per_minute()));

        bucket.try_consume().map_err(|retry_after| RateLimitError {
            retry_after_secs: retry_after,
            message: format!(
                "Rate limit exceeded for class {class:?}: max {} req/min",
                class.per_minute()
            ),
        })
    }

    /// Drain the bucket for testing: consume all tokens so the next call fails.
    #[cfg(test)]
    pub fn exhaust(&self, caller_id: &str, class: RateLimitClass) {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry((caller_id.to_string(), class))
            .or_insert_with(|| TokenBucket::new(class.per_minute()));
        bucket.last_refill = Instant::now();
        bucket.tokens = 0.0;
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_passes() {
        let limiter = RateLimiter::new();
        // AdminAction allows 1/min; first call should succeed
        assert!(limiter.check("user-1", RateLimitClass::AdminAction).is_ok());
    }

    #[test]
    fn over_limit_returns_rate_limited() {
        let limiter = RateLimiter::new();
        // Exhaust the bucket then verify next call is rejected
        limiter.exhaust("user-2", RateLimitClass::AdminAction);
        let result = limiter.check("user-2", RateLimitClass::AdminAction);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::RateLimited));
    }

    #[test]
    fn different_callers_have_independent_buckets() {
        let limiter = RateLimiter::new();
        limiter.exhaust("user-a", RateLimitClass::JobSubmit);
        // user-a is exhausted but user-b is not
        assert!(limiter.check("user-a", RateLimitClass::JobSubmit).is_err());
        assert!(limiter.check("user-b", RateLimitClass::JobSubmit).is_ok());
    }

    #[test]
    fn heartbeat_allows_120_per_minute() {
        assert_eq!(RateLimitClass::DonorHeartbeat.per_minute(), 120);
    }

    #[test]
    fn job_submit_allows_10_per_minute() {
        assert_eq!(RateLimitClass::JobSubmit.per_minute(), 10);
    }

    #[test]
    fn cluster_status_allows_30_per_minute() {
        assert_eq!(RateLimitClass::ClusterStatus.per_minute(), 30);
    }

    #[test]
    fn governance_allows_5_per_minute() {
        assert_eq!(RateLimitClass::Governance.per_minute(), 5);
    }

    #[test]
    fn try_acquire_succeeds_under_limit() {
        let limiter = RateLimiter::new();
        assert!(limiter.try_acquire(RateLimitClass::DonorHeartbeat, "node-1").is_ok());
        assert!(limiter.try_acquire(RateLimitClass::DonorHeartbeat, "node-1").is_ok());
    }

    #[test]
    fn try_acquire_returns_retry_after_on_exhaustion() {
        let limiter = RateLimiter::new();
        limiter.exhaust("node-x", RateLimitClass::AdminAction);
        let err = limiter.try_acquire(RateLimitClass::AdminAction, "node-x").unwrap_err();
        assert!(err.retry_after_secs > 0.0, "retry_after_secs should be positive");
        assert!(err.message.contains("Rate limit exceeded"));
    }
}
