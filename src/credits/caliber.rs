//! Caliber class definitions per FR-042 and data-model §3.17.
//!
//! Caliber class determines the hardware performance tier and enforces the
//! constitution's same-caliber redemption guarantee.

use serde::{Deserialize, Serialize};

/// Hardware performance tier classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CaliberClass {
    /// C0: Raspberry Pi, low-end ARM SBC
    C0 = 0,
    /// C1: Consumer laptop (4-8 cores, 8-16 GB RAM, no discrete GPU)
    C1 = 1,
    /// C2: Workstation (8-16 cores, 32-64 GB RAM, mid-range GPU)
    C2 = 2,
    /// C3: Server (16-64 cores, 128+ GB RAM, server-class GPU)
    C3 = 3,
    /// C4: High-end GPU (H100, A100, or equivalent)
    C4 = 4,
}

impl CaliberClass {
    /// Approximate NCU earn rate per hour for this caliber class.
    /// Used for credit normalization.
    pub fn ncu_per_hour(self) -> f64 {
        match self {
            Self::C0 => 0.01,
            Self::C1 => 0.1,
            Self::C2 => 1.0,
            Self::C3 => 10.0,
            Self::C4 => 100.0,
        }
    }

    /// Whether a redemption request for `requested` caliber can be served
    /// by `available` caliber. Per FR-042, the system guarantees same-caliber
    /// minimum, but a donor MAY voluntarily accept lower-tier with a 30% refund.
    pub fn can_serve(available: Self, requested: Self, voluntary_downgrade: bool) -> bool {
        if available >= requested {
            return true;
        }
        // Voluntary downgrade allowed with 30% NCU refund
        voluntary_downgrade
    }

    /// NCU refund ratio when a donor voluntarily accepts a lower caliber.
    pub const VOLUNTARY_DOWNGRADE_REFUND: f64 = 0.30;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_or_higher_caliber_always_serves() {
        assert!(CaliberClass::can_serve(CaliberClass::C3, CaliberClass::C2, false));
        assert!(CaliberClass::can_serve(CaliberClass::C4, CaliberClass::C4, false));
    }

    #[test]
    fn lower_caliber_rejected_without_voluntary() {
        assert!(!CaliberClass::can_serve(CaliberClass::C1, CaliberClass::C3, false));
    }

    #[test]
    fn lower_caliber_accepted_with_voluntary_downgrade() {
        assert!(CaliberClass::can_serve(CaliberClass::C1, CaliberClass::C3, true));
    }
}
