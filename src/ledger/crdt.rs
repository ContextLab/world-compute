//! CRDT OR-Map balance view for per-donor NCU balances.
//!
//! Maintains per-donor NCU balances derived from a LedgerEntry stream.
//! Uses a simple HashMap-based OR-Map semantics: entries from any replica
//! are merged by taking the union; balances are recomputed from the entry log.

use std::collections::HashMap;

use crate::ledger::entry::{LedgerEntry, LedgerEntryType};
use crate::types::NcuAmount;

/// A CRDT-based balance view over a stream of ledger entries.
///
/// OR-Map semantics: each entry is identified by its CID (unique). Merging
/// two replicas takes the union of their entry sets; balances are derived
/// by replaying all entries in sequence order.
#[derive(Debug, Clone, Default)]
pub struct BalanceView {
    /// All entries seen by this replica, keyed by entry_cid string.
    entries: HashMap<String, LedgerEntry>,
}

impl BalanceView {
    /// Create a new empty BalanceView.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a single ledger entry to this view.
    ///
    /// Idempotent: re-applying the same CID is a no-op.
    pub fn apply_entry(&mut self, entry: LedgerEntry) {
        let key = entry.entry_cid.to_string();
        self.entries.entry(key).or_insert(entry);
    }

    /// Merge another replica into this view (OR-Map union).
    ///
    /// After merging, this view contains all entries from both replicas.
    pub fn merge(&mut self, other: &BalanceView) {
        for (key, entry) in &other.entries {
            self.entries.entry(key.clone()).or_insert_with(|| entry.clone());
        }
    }

    /// Compute the current NCU balance for the given donor/subject ID.
    ///
    /// Returns `NcuAmount::ZERO` if the subject has no entries.
    /// Balance is floored at zero — it can never go negative.
    pub fn get_balance(&self, donor_id: &str) -> NcuAmount {
        // Collect entries for this subject and sort by sequence for determinism.
        let mut relevant: Vec<&LedgerEntry> =
            self.entries.values().filter(|e| e.subject_id == donor_id).collect();
        relevant.sort_by_key(|e| e.sequence);

        let mut balance: i64 = 0;
        for entry in relevant {
            match entry.entry_type {
                LedgerEntryType::CreditEarn | LedgerEntryType::CreditRefund => {
                    balance = balance.saturating_add(entry.ncu_delta.abs());
                }
                LedgerEntryType::CreditSpend | LedgerEntryType::CreditDecay => {
                    balance = balance.saturating_sub(entry.ncu_delta.abs());
                }
                // Governance and audit records don't affect balance.
                LedgerEntryType::GovernanceRecord | LedgerEntryType::AuditRecord => {}
            }
            // Floor at zero.
            if balance < 0 {
                balance = 0;
            }
        }

        NcuAmount(balance as u64)
    }

    /// Number of entries in this view.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::entry::LedgerEntryType;
    use crate::types::{NcuAmount, SignatureBundle, Timestamp};
    use cid::Cid;
    use multihash::Multihash;
    use sha2::{Digest, Sha256};

    fn dummy_sig() -> SignatureBundle {
        SignatureBundle {
            signer_ids: vec!["coord-1".into()],
            signature: vec![0u8; 64],
            threshold: 1,
            total: 1,
        }
    }

    fn make_cid(seed: u8) -> Cid {
        let hash = Sha256::digest([seed]);
        let mh = Multihash::<64>::wrap(0x12, &hash).unwrap();
        Cid::new_v1(0x55, mh)
    }

    fn make_entry(
        cid_seed: u8,
        subject: &str,
        entry_type: LedgerEntryType,
        ncu_delta: i64,
        sequence: u64,
    ) -> LedgerEntry {
        LedgerEntry {
            entry_cid: make_cid(cid_seed),
            prev_cid: None,
            sequence,
            entry_type,
            timestamp: Timestamp::now(),
            subject_id: subject.to_string(),
            ncu_delta,
            payload: vec![],
            signature: dummy_sig(),
        }
    }

    #[test]
    fn test_apply_earn_increases_balance() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "alice", LedgerEntryType::CreditEarn, 1000, 0));
        assert_eq!(view.get_balance("alice"), NcuAmount(1000));
    }

    #[test]
    fn test_apply_spend_decreases_balance() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "alice", LedgerEntryType::CreditEarn, 2000, 0));
        view.apply_entry(make_entry(2, "alice", LedgerEntryType::CreditSpend, 500, 1));
        assert_eq!(view.get_balance("alice"), NcuAmount(1500));
    }

    #[test]
    fn test_apply_decay() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "bob", LedgerEntryType::CreditEarn, 1000, 0));
        view.apply_entry(make_entry(2, "bob", LedgerEntryType::CreditDecay, 100, 1));
        assert_eq!(view.get_balance("bob"), NcuAmount(900));
    }

    #[test]
    fn test_apply_refund_increases_balance() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "carol", LedgerEntryType::CreditEarn, 500, 0));
        view.apply_entry(make_entry(2, "carol", LedgerEntryType::CreditSpend, 500, 1));
        view.apply_entry(make_entry(3, "carol", LedgerEntryType::CreditRefund, 200, 2));
        assert_eq!(view.get_balance("carol"), NcuAmount(200));
    }

    #[test]
    fn test_balance_never_negative() {
        let mut view = BalanceView::new();
        // Spend more than earned — should floor at zero.
        view.apply_entry(make_entry(1, "dave", LedgerEntryType::CreditEarn, 100, 0));
        view.apply_entry(make_entry(2, "dave", LedgerEntryType::CreditSpend, 1000, 1));
        assert_eq!(view.get_balance("dave"), NcuAmount::ZERO);
    }

    #[test]
    fn test_unknown_subject_returns_zero() {
        let view = BalanceView::new();
        assert_eq!(view.get_balance("nobody"), NcuAmount::ZERO);
    }

    #[test]
    fn test_merge_two_replicas() {
        let mut replica_a = BalanceView::new();
        replica_a.apply_entry(make_entry(1, "eve", LedgerEntryType::CreditEarn, 1000, 0));

        let mut replica_b = BalanceView::new();
        replica_b.apply_entry(make_entry(2, "eve", LedgerEntryType::CreditEarn, 500, 1));

        // Merge B into A.
        replica_a.merge(&replica_b);
        assert_eq!(replica_a.get_balance("eve"), NcuAmount(1500));

        // Merge A into B and verify symmetry.
        let mut replica_b2 = BalanceView::new();
        replica_b2.apply_entry(make_entry(2, "eve", LedgerEntryType::CreditEarn, 500, 1));
        let mut replica_a2 = BalanceView::new();
        replica_a2.apply_entry(make_entry(1, "eve", LedgerEntryType::CreditEarn, 1000, 0));
        replica_b2.merge(&replica_a2);
        assert_eq!(replica_b2.get_balance("eve"), NcuAmount(1500));
    }

    #[test]
    fn test_merge_idempotent() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "frank", LedgerEntryType::CreditEarn, 300, 0));

        let clone = view.clone();
        view.merge(&clone);
        // Should still be 300, not 600.
        assert_eq!(view.get_balance("frank"), NcuAmount(300));
        assert_eq!(view.entry_count(), 1);
    }

    #[test]
    fn test_governance_record_does_not_affect_balance() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "grace", LedgerEntryType::CreditEarn, 500, 0));
        view.apply_entry(make_entry(2, "grace", LedgerEntryType::GovernanceRecord, 9999, 1));
        assert_eq!(view.get_balance("grace"), NcuAmount(500));
    }

    #[test]
    fn test_isolated_subject_balances() {
        let mut view = BalanceView::new();
        view.apply_entry(make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0));
        view.apply_entry(make_entry(2, "bob", LedgerEntryType::CreditEarn, 200, 0));
        assert_eq!(view.get_balance("alice"), NcuAmount(100));
        assert_eq!(view.get_balance("bob"), NcuAmount(200));
    }
}
