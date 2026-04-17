//! Integration tests for CRDT merge, balance verification, and graceful degradation (T139-T143).

use std::collections::HashMap;
use worldcompute::ledger::crdt::{
    cache_lease_offers, compute_merkle_root, merge_or_maps, queue_ledger_write, verify_balance,
};
use worldcompute::ledger::entry::{LedgerEntry, LedgerEntryType};
use worldcompute::types::{NcuAmount, SignatureBundle, Timestamp};

fn dummy_sig() -> SignatureBundle {
    SignatureBundle {
        signer_ids: vec!["coord-1".into()],
        signature: vec![0u8; 64],
        threshold: 1,
        total: 1,
    }
}

fn make_cid(seed: u8) -> cid::Cid {
    use multihash::Multihash;
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest([seed]);
    let mh = Multihash::<64>::wrap(0x12, &hash).unwrap();
    cid::Cid::new_v1(0x55, mh)
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
fn merge_or_maps_last_writer_wins() {
    let mut local: HashMap<String, LedgerEntry> = HashMap::new();
    let mut remote: HashMap<String, LedgerEntry> = HashMap::new();

    let mut e1 = make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0);
    e1.timestamp = Timestamp(1000);
    local.insert("key1".to_string(), e1);

    let mut e2 = make_entry(2, "alice", LedgerEntryType::CreditEarn, 200, 0);
    e2.timestamp = Timestamp(2000); // newer
    remote.insert("key1".to_string(), e2);

    // Remote has new key
    let e3 = make_entry(3, "bob", LedgerEntryType::CreditEarn, 300, 0);
    remote.insert("key2".to_string(), e3);

    merge_or_maps(&mut local, &remote);

    // key1 should have remote's value (newer timestamp)
    assert_eq!(local["key1"].ncu_delta, 200);
    // key2 should be inserted from remote
    assert!(local.contains_key("key2"));
    assert_eq!(local["key2"].ncu_delta, 300);
}

#[test]
fn merge_or_maps_local_wins_when_newer() {
    let mut local: HashMap<String, LedgerEntry> = HashMap::new();
    let mut remote: HashMap<String, LedgerEntry> = HashMap::new();

    let mut e1 = make_entry(1, "alice", LedgerEntryType::CreditEarn, 500, 0);
    e1.timestamp = Timestamp(5000); // newer
    local.insert("key1".to_string(), e1);

    let mut e2 = make_entry(2, "alice", LedgerEntryType::CreditEarn, 100, 0);
    e2.timestamp = Timestamp(1000);
    remote.insert("key1".to_string(), e2);

    merge_or_maps(&mut local, &remote);

    // Local should keep its value (newer)
    assert_eq!(local["key1"].ncu_delta, 500);
}

#[test]
fn compute_merkle_root_deterministic() {
    let mut entries: HashMap<String, LedgerEntry> = HashMap::new();
    entries.insert("a".to_string(), make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0));
    entries.insert("b".to_string(), make_entry(2, "bob", LedgerEntryType::CreditEarn, 200, 1));

    let root1 = compute_merkle_root(&entries);
    let root2 = compute_merkle_root(&entries);
    assert_eq!(root1, root2, "Merkle root should be deterministic");
    assert_eq!(root1.len(), 32, "SHA-256 hash should be 32 bytes");
}

#[test]
fn compute_merkle_root_changes_with_data() {
    let mut entries1: HashMap<String, LedgerEntry> = HashMap::new();
    entries1.insert("a".to_string(), make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0));

    let mut entries2: HashMap<String, LedgerEntry> = HashMap::new();
    entries2.insert("a".to_string(), make_entry(2, "bob", LedgerEntryType::CreditEarn, 200, 0));

    let root1 = compute_merkle_root(&entries1);
    let root2 = compute_merkle_root(&entries2);
    assert_ne!(root1, root2, "Different entries should produce different roots");
}

#[test]
fn verify_balance_correct() {
    let entries = vec![
        make_entry(1, "alice", LedgerEntryType::CreditEarn, 1000, 0),
        make_entry(2, "alice", LedgerEntryType::CreditSpend, 300, 1),
    ];
    assert!(verify_balance(&entries, NcuAmount(700)));
    assert!(!verify_balance(&entries, NcuAmount(1000)));
}

#[test]
fn verify_balance_with_decay() {
    let entries = vec![
        make_entry(1, "alice", LedgerEntryType::CreditEarn, 1000, 0),
        make_entry(2, "alice", LedgerEntryType::CreditDecay, 100, 1),
    ];
    assert!(verify_balance(&entries, NcuAmount(900)));
}

#[test]
fn verify_balance_never_negative() {
    let entries = vec![
        make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0),
        make_entry(2, "alice", LedgerEntryType::CreditSpend, 500, 1),
    ];
    assert!(verify_balance(&entries, NcuAmount(0)));
}

#[test]
fn cache_lease_offers_returns_copy() {
    let offers = vec![
        ("lease-1".to_string(), "node-a".to_string()),
        ("lease-2".to_string(), "node-b".to_string()),
    ];
    let cached = cache_lease_offers(&offers);
    assert_eq!(cached.len(), 2);
    assert_eq!(cached[0].0, "lease-1");
}

#[test]
fn queue_ledger_write_appends() {
    let mut queue: Vec<LedgerEntry> = Vec::new();
    let e1 = make_entry(1, "alice", LedgerEntryType::CreditEarn, 100, 0);
    let e2 = make_entry(2, "bob", LedgerEntryType::CreditEarn, 200, 1);

    assert_eq!(queue_ledger_write(&mut queue, e1), 1);
    assert_eq!(queue_ledger_write(&mut queue, e2), 2);
    assert_eq!(queue.len(), 2);
}
