//! Integration tests for CRDT ledger operations (T105).

use cid::Cid;
use multihash::Multihash;
use sha2::{Digest, Sha256};
use worldcompute::ledger::crdt::BalanceView;
use worldcompute::ledger::entry::{LedgerEntry, LedgerEntryType};
use worldcompute::ledger::{LedgerShard, MerkleRoot};
use worldcompute::types::{NcuAmount, SignatureBundle, Timestamp};

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
fn ledger_entry_creation() {
    let entry = make_entry(1, "alice", LedgerEntryType::CreditEarn, 1000, 0);
    assert_eq!(entry.subject_id, "alice");
    assert_eq!(entry.ncu_delta, 1000);
    assert_eq!(entry.entry_type, LedgerEntryType::CreditEarn);
}

#[test]
fn merkle_chain_linking() {
    let entry1 = make_entry(1, "bob", LedgerEntryType::CreditEarn, 500, 0);
    let mut entry2 = make_entry(2, "bob", LedgerEntryType::CreditSpend, 200, 1);
    entry2.prev_cid = Some(entry1.entry_cid);

    assert!(entry2.prev_cid.is_some());
    assert_eq!(entry2.prev_cid.unwrap(), entry1.entry_cid);
}

#[test]
fn entry_type_variants() {
    let types = [
        LedgerEntryType::CreditEarn,
        LedgerEntryType::CreditSpend,
        LedgerEntryType::CreditDecay,
        LedgerEntryType::CreditRefund,
        LedgerEntryType::GovernanceRecord,
        LedgerEntryType::AuditRecord,
    ];
    // All variants should be distinct
    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            assert_ne!(types[i], types[j]);
        }
    }
}

#[test]
fn crdt_balance_earn_and_spend() {
    let mut view = BalanceView::new();
    view.apply_entry(make_entry(1, "carol", LedgerEntryType::CreditEarn, 2000, 0));
    view.apply_entry(make_entry(2, "carol", LedgerEntryType::CreditSpend, 500, 1));
    assert_eq!(view.get_balance("carol"), NcuAmount(1500));
}

#[test]
fn ledger_shard_creation() {
    let shard = LedgerShard {
        shard_id: "shard-001".into(),
        coordinator_id: "coord-001".into(),
        head_cid: make_cid(99),
        head_sequence: 42,
        head_timestamp: Timestamp::now(),
    };
    assert_eq!(shard.shard_id, "shard-001");
    assert_eq!(shard.head_sequence, 42);
}

#[test]
fn merkle_root_with_shard_heads() {
    let root = MerkleRoot {
        root_hash: vec![0u8; 32],
        height: 10,
        timestamp: Timestamp::now(),
        shard_heads: vec![LedgerShard {
            shard_id: "s1".into(),
            coordinator_id: "c1".into(),
            head_cid: make_cid(1),
            head_sequence: 5,
            head_timestamp: Timestamp::now(),
        }],
        coordinator_signature: dummy_sig(),
        rekor_entry_id: None,
    };
    assert_eq!(root.height, 10);
    assert_eq!(root.shard_heads.len(), 1);
}
