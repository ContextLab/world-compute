//! Quadratic voting budget per US6.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

/// Default per-epoch vote budget (in quadratic cost units).
pub const DEFAULT_EPOCH_BUDGET: u32 = 20;

/// Per-user quadratic vote budget for the current epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuadraticVoteBudget {
    /// Total budget for this epoch.
    pub epoch_budget: u32,
    /// Cumulative cost of votes cast so far this epoch.
    pub votes_cast: u32,
    /// Epoch start timestamp (used to detect epoch resets).
    pub epoch_start: Timestamp,
}

impl QuadraticVoteBudget {
    /// Create a new budget with the default epoch budget.
    pub fn new() -> Self {
        Self { epoch_budget: DEFAULT_EPOCH_BUDGET, votes_cast: 0, epoch_start: Timestamp::now() }
    }

    /// Quadratic cost for casting a vote of given weight: cost = weight².
    pub fn cast_cost(weight: u32) -> u32 {
        weight.saturating_mul(weight)
    }

    /// Returns true if the user can afford the cost of a vote with this weight.
    pub fn can_afford(&self, weight: u32) -> bool {
        let cost = Self::cast_cost(weight);
        self.votes_cast.saturating_add(cost) <= self.epoch_budget
    }

    /// Deduct the cost of a vote with the given weight from the budget.
    /// Returns the remaining budget on success.
    pub fn apply_vote(&mut self, weight: u32) -> WcResult<u32> {
        if !self.can_afford(weight) {
            return Err(WcError::new(
                ErrorCode::InsufficientCredits,
                format!(
                    "quadratic vote weight {} costs {} but only {} budget remaining",
                    weight,
                    Self::cast_cost(weight),
                    self.epoch_budget.saturating_sub(self.votes_cast)
                ),
            ));
        }
        self.votes_cast = self.votes_cast.saturating_add(Self::cast_cost(weight));
        Ok(self.epoch_budget.saturating_sub(self.votes_cast))
    }

    /// Reset the budget for a new epoch.
    pub fn reset_epoch(&mut self) {
        self.votes_cast = 0;
        self.epoch_start = Timestamp::now();
    }

    /// Remaining budget in cost units.
    pub fn remaining(&self) -> u32 {
        self.epoch_budget.saturating_sub(self.votes_cast)
    }
}

impl Default for QuadraticVoteBudget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_scales_quadratically() {
        assert_eq!(QuadraticVoteBudget::cast_cost(1), 1);
        assert_eq!(QuadraticVoteBudget::cast_cost(2), 4);
        assert_eq!(QuadraticVoteBudget::cast_cost(3), 9);
        assert_eq!(QuadraticVoteBudget::cast_cost(4), 16);
    }

    #[test]
    fn can_afford_within_budget() {
        let budget = QuadraticVoteBudget::new(); // 20 budget
        assert!(budget.can_afford(4)); // cost 16 <= 20
        assert!(!budget.can_afford(5)); // cost 25 > 20
    }

    #[test]
    fn apply_vote_deducts_correctly() {
        let mut budget = QuadraticVoteBudget::new();
        let remaining = budget.apply_vote(2).unwrap(); // cost 4
        assert_eq!(remaining, 16);
        let remaining2 = budget.apply_vote(2).unwrap(); // cost 4 again
        assert_eq!(remaining2, 12);
    }

    #[test]
    fn budget_exhaustion() {
        let mut budget = QuadraticVoteBudget::new();
        budget.apply_vote(4).unwrap(); // cost 16, 4 remaining
        let err = budget.apply_vote(3).unwrap_err(); // cost 9 > 4
        assert_eq!(err.code(), Some(ErrorCode::InsufficientCredits));
    }

    #[test]
    fn epoch_reset_restores_budget() {
        let mut budget = QuadraticVoteBudget::new();
        budget.apply_vote(4).unwrap(); // cost 16
        assert_eq!(budget.remaining(), 4);
        budget.reset_epoch();
        assert_eq!(budget.remaining(), 20);
    }

    #[test]
    fn weight_1_costs_1() {
        let mut budget = QuadraticVoteBudget::new();
        // Can cast 20 weight-1 votes (each costs 1)
        for _ in 0..20 {
            budget.apply_vote(1).unwrap();
        }
        assert_eq!(budget.remaining(), 0);
        assert!(budget.apply_vote(1).is_err());
    }
}
