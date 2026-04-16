//! Multi-factor priority scoring per FR-032 (T060).
//!
//! P(job) = 0.35·S_ncu + 0.25·S_vote + 0.15·S_size + 0.15·S_age + 0.10·S_cool
//!
//! All signals normalized to [0,1]. No job is ever permanently blocked.

use crate::credits::ncu::{compute_priority_s_ncu, DEFAULT_ALPHA};
use crate::types::NcuAmount;

/// Inputs to the priority score computation.
#[derive(Debug, Clone)]
pub struct PriorityInputs {
    /// Submitter's NCU balance.
    pub ncu_balance: NcuAmount,
    /// Net votes from verified humans (can be negative).
    pub net_votes: i64,
    /// Total verified voters who have voted on this job's proposal.
    pub total_voters: u64,
    /// Requested resource size (normalized: 0.0 = tiny, 1.0 = maximum).
    pub size_fraction: f64,
    /// Time in queue in seconds.
    pub queue_age_seconds: f64,
    /// Submitter's total compute consumed in trailing 24h window (in NCU).
    pub trailing_24h_ncu: f64,
}

/// Priority weights per FR-032.
const W_NCU: f64 = 0.35;
const W_VOTE: f64 = 0.25;
const W_SIZE: f64 = 0.15;
const W_AGE: f64 = 0.15;
const W_COOL: f64 = 0.10;

/// Age half-life: 4 hours (14,400 seconds).
/// After 4 hours, S_age reaches ~0.5; after ~28 hours, ~0.99.
const AGE_HALF_LIFE_SECONDS: f64 = 14_400.0;

/// Cooldown half-life: 24 hours of trailing NCU consumption.
const COOL_HALF_LIFE_NCU: f64 = 10.0;

/// Compute the composite priority score for a job.
/// Returns a value in [0.0, 1.0]. Higher is higher priority.
pub fn compute_priority(inputs: &PriorityInputs) -> f64 {
    let s_ncu = compute_priority_s_ncu(inputs.ncu_balance, DEFAULT_ALPHA);
    let s_vote = compute_s_vote(inputs.net_votes, inputs.total_voters);
    let s_size = compute_s_size(inputs.size_fraction);
    let s_age = compute_s_age(inputs.queue_age_seconds);
    let s_cool = compute_s_cool(inputs.trailing_24h_ncu);

    let score = W_NCU * s_ncu + W_VOTE * s_vote + W_SIZE * s_size + W_AGE * s_age + W_COOL * s_cool;

    score.clamp(0.0, 1.0)
}

/// S_vote: population-normalized public importance vote score.
/// tanh(net_votes / sqrt(total_voters + 1)) mapped to [0, 1].
fn compute_s_vote(net_votes: i64, total_voters: u64) -> f64 {
    if total_voters == 0 && net_votes == 0 {
        return 0.5; // Neutral — no votes cast
    }
    let normalized = net_votes as f64 / (total_voters as f64 + 1.0).sqrt();
    (normalized.tanh() + 1.0) / 2.0 // Map tanh [-1,1] to [0,1]
}

/// S_size: exponential decay penalizing larger jobs.
/// Small/short jobs get higher priority (Slurm-style backfill).
fn compute_s_size(size_fraction: f64) -> f64 {
    (-2.0 * size_fraction.clamp(0.0, 1.0)).exp()
}

/// S_age: exponential saturation ensuring starvation freedom.
/// 1 - exp(-ln2 * t / half_life). Reaches 0.5 at 4 hours, ~1.0 at ~28 hours.
fn compute_s_age(queue_age_seconds: f64) -> f64 {
    let t = queue_age_seconds.max(0.0);
    1.0 - (-std::f64::consts::LN_2 * t / AGE_HALF_LIFE_SECONDS).exp()
}

/// S_cool: exponential decay penalizing recent heavy usage.
/// Users who recently consumed lots of NCU have lower priority.
fn compute_s_cool(trailing_24h_ncu: f64) -> f64 {
    (-trailing_24h_ncu.max(0.0) / COOL_HALF_LIFE_NCU).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_everything_gives_moderate_score() {
        let inputs = PriorityInputs {
            ncu_balance: NcuAmount::ZERO,
            net_votes: 0,
            total_voters: 0,
            size_fraction: 0.5,
            queue_age_seconds: 0.0,
            trailing_24h_ncu: 0.0,
        };
        let score = compute_priority(&inputs);
        // Should be moderate: S_vote=0.5, S_size~0.37, S_cool=1.0, S_ncu=0, S_age=0
        assert!(score > 0.1 && score < 0.5, "Score: {score}");
    }

    #[test]
    fn rich_donor_gets_high_priority() {
        let inputs = PriorityInputs {
            ncu_balance: NcuAmount::from_ncu(100.0),
            net_votes: 10,
            total_voters: 20,
            size_fraction: 0.1,
            queue_age_seconds: 0.0,
            trailing_24h_ncu: 0.0,
        };
        let score = compute_priority(&inputs);
        assert!(score > 0.7, "Rich donor score: {score}");
    }

    #[test]
    fn old_job_eventually_gets_high_priority() {
        let inputs = PriorityInputs {
            ncu_balance: NcuAmount::ZERO,
            net_votes: 0,
            total_voters: 0,
            size_fraction: 0.5,
            queue_age_seconds: 8.0 * 3600.0, // 8 hours
            trailing_24h_ncu: 0.0,
        };
        let score = compute_priority(&inputs);
        // S_age after 8 hours should be significant
        assert!(score > 0.3, "8-hour-old job score: {score}");
    }

    #[test]
    fn heavy_user_gets_cooldown_penalty() {
        let base = PriorityInputs {
            ncu_balance: NcuAmount::from_ncu(10.0),
            net_votes: 0,
            total_voters: 0,
            size_fraction: 0.3,
            queue_age_seconds: 0.0,
            trailing_24h_ncu: 0.0,
        };
        let heavy = PriorityInputs { trailing_24h_ncu: 50.0, ..base.clone() };
        let s_base = compute_priority(&base);
        let s_heavy = compute_priority(&heavy);
        assert!(s_heavy < s_base, "Heavy user ({s_heavy}) should be < fresh user ({s_base})");
    }

    #[test]
    fn small_jobs_prioritized_over_large() {
        let small = PriorityInputs {
            ncu_balance: NcuAmount::ZERO,
            net_votes: 0,
            total_voters: 0,
            size_fraction: 0.05,
            queue_age_seconds: 0.0,
            trailing_24h_ncu: 0.0,
        };
        let large = PriorityInputs { size_fraction: 0.95, ..small.clone() };
        let s_small = compute_priority(&small);
        let s_large = compute_priority(&large);
        assert!(s_small > s_large, "Small ({s_small}) should beat large ({s_large})");
    }
}
