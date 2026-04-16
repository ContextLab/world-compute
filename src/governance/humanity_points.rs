//! Humanity Points (HP) — sybil-resistance scoring per US6 / FR-059.

use serde::{Deserialize, Serialize};

/// Full vote weight threshold in HP.
const FULL_WEIGHT_HP: u32 = 5;

/// Humanity Points record for a single user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HumanityPoints {
    /// +1 HP
    pub email_verified: bool,
    /// +3 HP
    pub phone_verified: bool,
    /// +2 HP each, capped at 3 accounts
    pub social_accounts: u8,
    /// +2 HP each, capped at 3 vouches
    pub web_of_trust_vouches: u8,
    /// +3 HP
    pub proof_of_personhood: bool,
    /// +5 HP
    pub active_donor: bool,
}

impl HumanityPoints {
    /// Compute total earned HP.
    pub fn compute_hp(&self) -> u32 {
        let mut hp: u32 = 0;
        if self.email_verified {
            hp += 1;
        }
        if self.phone_verified {
            hp += 3;
        }
        hp += 2 * (self.social_accounts.min(3) as u32);
        hp += 2 * (self.web_of_trust_vouches.min(3) as u32);
        if self.proof_of_personhood {
            hp += 3;
        }
        if self.active_donor {
            hp += 5;
        }
        hp
    }

    /// Returns true when HP >= 5 (full vote weight).
    pub fn has_full_vote_weight(&self) -> bool {
        self.compute_hp() >= FULL_WEIGHT_HP
    }

    /// Vote weight fraction: min(1.0, HP / 5.0).
    pub fn vote_weight_fraction(&self) -> f64 {
        let hp = self.compute_hp() as f64;
        (hp / FULL_WEIGHT_HP as f64).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_hp() {
        let hp = HumanityPoints::default();
        assert_eq!(hp.compute_hp(), 0);
        assert!(!hp.has_full_vote_weight());
        assert_eq!(hp.vote_weight_fraction(), 0.0);
    }

    #[test]
    fn email_only() {
        let hp = HumanityPoints { email_verified: true, ..Default::default() };
        assert_eq!(hp.compute_hp(), 1);
        assert!(!hp.has_full_vote_weight());
    }

    #[test]
    fn partial_hp_fraction() {
        // phone (3) + email (1) = 4 HP → 0.8 fraction
        let hp =
            HumanityPoints { email_verified: true, phone_verified: true, ..Default::default() };
        assert_eq!(hp.compute_hp(), 4);
        assert!(!hp.has_full_vote_weight());
        let frac = hp.vote_weight_fraction();
        assert!((frac - 0.8).abs() < 1e-9);
    }

    #[test]
    fn full_hp_from_active_donor() {
        let hp = HumanityPoints { active_donor: true, ..Default::default() };
        assert_eq!(hp.compute_hp(), 5);
        assert!(hp.has_full_vote_weight());
        assert_eq!(hp.vote_weight_fraction(), 1.0);
    }

    #[test]
    fn social_accounts_capped_at_3() {
        let hp = HumanityPoints { social_accounts: 10, ..Default::default() };
        // Max contribution from social: 2*3 = 6
        assert_eq!(hp.compute_hp(), 6);
    }

    #[test]
    fn web_of_trust_vouches_capped_at_3() {
        let hp = HumanityPoints { web_of_trust_vouches: 10, ..Default::default() };
        assert_eq!(hp.compute_hp(), 6);
    }

    #[test]
    fn max_hp() {
        // email(1) + phone(3) + 3 social(6) + 3 vouches(6) + personhood(3) + donor(5) = 24
        let hp = HumanityPoints {
            email_verified: true,
            phone_verified: true,
            social_accounts: 3,
            web_of_trust_vouches: 3,
            proof_of_personhood: true,
            active_donor: true,
        };
        assert_eq!(hp.compute_hp(), 24);
        assert!(hp.has_full_vote_weight());
        assert_eq!(hp.vote_weight_fraction(), 1.0);
    }
}
