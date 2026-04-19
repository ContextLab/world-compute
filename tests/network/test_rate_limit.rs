//! Integration tests for token-bucket rate limiting (T094).

use worldcompute::network::rate_limit::{RateLimitClass, RateLimiter};

#[test]
fn donor_heartbeat_accepts_two_requests() {
    let limiter = RateLimiter::new();
    // DonorHeartbeat allows 120/min = 2/sec; two immediate calls should succeed
    let r1 = limiter.try_acquire(RateLimitClass::DonorHeartbeat, "donor-1");
    let r2 = limiter.try_acquire(RateLimitClass::DonorHeartbeat, "donor-1");
    assert!(r1.is_ok(), "first heartbeat request should be accepted");
    assert!(r2.is_ok(), "second heartbeat request should be accepted");
}

#[test]
fn job_submit_burst_then_reject() {
    let limiter = RateLimiter::new();
    // JobSubmit allows 10/min burst. Exhaust all tokens.
    for i in 0..10 {
        let result = limiter.try_acquire(RateLimitClass::JobSubmit, "submitter-1");
        assert!(result.is_ok(), "request {i} within burst should succeed");
    }
    // 11th request should be rejected
    let result = limiter.try_acquire(RateLimitClass::JobSubmit, "submitter-1");
    assert!(result.is_err(), "request over burst limit should be rejected");
}

#[test]
fn retry_after_secs_is_populated_on_rejection() {
    let limiter = RateLimiter::new();
    // AdminAction allows 1/min — exhaust it
    limiter.try_acquire(RateLimitClass::AdminAction, "admin-1").unwrap();
    let err = limiter
        .try_acquire(RateLimitClass::AdminAction, "admin-1")
        .expect_err("should be rejected after burst");
    assert!(
        err.retry_after_secs > 0.0,
        "retry_after_secs should be positive, got {}",
        err.retry_after_secs
    );
}

#[test]
fn different_keys_have_independent_buckets() {
    let limiter = RateLimiter::new();
    // Exhaust admin-a
    limiter.try_acquire(RateLimitClass::AdminAction, "admin-a").unwrap();
    assert!(
        limiter.try_acquire(RateLimitClass::AdminAction, "admin-a").is_err(),
        "admin-a should be exhausted"
    );
    // admin-b should still work
    assert!(
        limiter.try_acquire(RateLimitClass::AdminAction, "admin-b").is_ok(),
        "admin-b should have its own bucket"
    );
}

#[test]
fn governance_allows_5_burst() {
    let limiter = RateLimiter::new();
    for _ in 0..5 {
        assert!(limiter.try_acquire(RateLimitClass::Governance, "voter-1").is_ok());
    }
    assert!(
        limiter.try_acquire(RateLimitClass::Governance, "voter-1").is_err(),
        "6th governance request should be rejected"
    );
}
