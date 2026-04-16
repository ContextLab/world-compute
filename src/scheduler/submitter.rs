//! Submitter entity per FR-103 (T070).
//!
//! A Submitter represents an entity that submits jobs to the cluster.
//! Per FR-103: submitter attributes do NOT affect scheduling priority —
//! scheduling is governed solely by the priority score system (FR-032).

use crate::types::NcuAmount;

/// Standing of the submitter with respect to acceptable-use policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptableUseStanding {
    /// No violations on record.
    Good,
    /// Under review — new jobs may be held pending review outcome.
    UnderReview,
    /// Suspended — job submission is blocked.
    Suspended,
}

/// A submitter entity: the party that requests compute work.
///
/// Invariant (FR-103): no field on this struct participates in scheduling
/// priority. Priority is determined entirely by the continuous multi-factor
/// score defined in `scheduler::priority`.
#[derive(Debug, Clone)]
pub struct Submitter {
    /// Unique submitter identifier (derived from Ed25519 public key).
    pub submitter_id: String,
    /// Current NCU credit balance available for job payment.
    pub credit_balance: NcuAmount,
    /// Acceptable-use policy standing.
    pub acceptable_use_standing: AcceptableUseStanding,
    /// Total lifetime jobs submitted (informational, not used for priority).
    pub total_jobs_submitted: u64,
}

impl Submitter {
    /// Create a new submitter with zero balance and clean standing.
    pub fn new(submitter_id: impl Into<String>) -> Self {
        Self {
            submitter_id: submitter_id.into(),
            credit_balance: NcuAmount::ZERO,
            acceptable_use_standing: AcceptableUseStanding::Good,
            total_jobs_submitted: 0,
        }
    }

    /// Returns true if the submitter is allowed to submit new jobs.
    pub fn can_submit(&self) -> bool {
        self.acceptable_use_standing != AcceptableUseStanding::Suspended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submitter_creation_with_zero_balance() {
        let s = Submitter::new("did:wc:abc123");
        assert_eq!(s.submitter_id, "did:wc:abc123");
        assert_eq!(s.credit_balance, NcuAmount::ZERO);
        assert_eq!(s.acceptable_use_standing, AcceptableUseStanding::Good);
        assert_eq!(s.total_jobs_submitted, 0);
    }

    #[test]
    fn new_submitter_can_submit() {
        let s = Submitter::new("did:wc:submitter-1");
        assert!(s.can_submit());
    }

    #[test]
    fn suspended_submitter_cannot_submit() {
        let mut s = Submitter::new("did:wc:bad-actor");
        s.acceptable_use_standing = AcceptableUseStanding::Suspended;
        assert!(!s.can_submit());
    }

    #[test]
    fn under_review_submitter_can_still_submit() {
        let mut s = Submitter::new("did:wc:under-review");
        s.acceptable_use_standing = AcceptableUseStanding::UnderReview;
        assert!(s.can_submit());
    }

    #[test]
    fn credit_balance_arithmetic() {
        let mut s = Submitter::new("did:wc:rich");
        s.credit_balance = NcuAmount::from_ncu(50.0);
        let deducted = s.credit_balance.saturating_sub(NcuAmount::from_ncu(10.0));
        assert!((deducted.as_ncu() - 40.0).abs() < 0.001);
    }
}
