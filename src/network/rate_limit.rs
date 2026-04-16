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
    GovernanceVote,
    /// Administrative actions: 1 per minute.
    AdminAction,
}

impl RateLimitClass {
    /// Maximum requests allowed per minute for this class.
    pub fn per_minute(self) -> u32 {
        match self {
            Self::DonorHeartbeat => 120,
            Self::JobSubmit => 10,
            Self::GovernanceVote => 5,
            Self::AdminAction => 1,
        }
    }
}

/// Per-class token bucket state.
#[derive(Debug)]
struct Bucket {
    tokens: f64,
    capacity: f64,
    /// Tokens added per second.
    refill_rate: f64,
    last_refill: Instant,
}

impl Bucket {
    fn new(per_minute: u32) -> Self {
        let capacity = per_minute as f64;
        Self {
            tokens: capacity,
            capacity,
            refill_rate: capacity / 60.0,
            last_refill: Instant::now(),
        }
    }

    /// Attempt to consume one token. Returns true if successful.
    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Token-bucket rate limiter keyed by `(caller_id, RateLimitClass)`.
///
/// A single `RateLimiter` instance is shared across the process. Each unique
/// (caller, class) pair has an independent bucket.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<(String, RateLimitClass), Bucket>>>,
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
            .or_insert_with(|| Bucket::new(class.per_minute()));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(WcError::new(
                ErrorCode::RateLimited,
                format!(
                    "Rate limit exceeded for class {class:?}: max {} req/min",
                    class.per_minute()
                ),
            ))
        }
    }

    /// Drain the bucket for testing: consume all tokens so the next call fails.
    #[cfg(test)]
    pub fn exhaust(&self, caller_id: &str, class: RateLimitClass) {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry((caller_id.to_string(), class))
            .or_insert_with(|| Bucket::new(class.per_minute()));
        // Set last_refill far in the past then drain tokens
        bucket.last_refill = Instant::now() - std::time::Duration::from_secs(0);
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
}
