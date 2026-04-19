//! Integration tests for storage GC and cap tracking (T127-T131).

use worldcompute::acceptable_use::filter::classify_workload;
use worldcompute::data_plane::cid_store::{garbage_collect, track_storage, CidStore, StorageCap};
use worldcompute::data_plane::placement::check_shard_residency;
use worldcompute::types::Timestamp;

fn make_cap(cap_bytes: u64) -> StorageCap {
    StorageCap {
        node_id: libp2p::PeerId::random(),
        cap_bytes,
        used_bytes: 0,
        last_gc_at: Timestamp::now(),
    }
}

#[test]
fn track_storage_within_cap() {
    let mut cap = make_cap(1000);
    assert!(track_storage(&mut cap, 500).is_ok());
    assert_eq!(cap.used_bytes, 500);
}

#[test]
fn track_storage_exceeds_cap_rejected() {
    let mut cap = make_cap(1000);
    track_storage(&mut cap, 800).unwrap();
    let result = track_storage(&mut cap, 300);
    assert!(result.is_err(), "Should reject when exceeding cap");
}

#[test]
fn fill_cap_gc_then_accept() {
    // T131: Fill cap -> reject -> GC -> accept
    let store = CidStore::new();
    let mut cap = make_cap(200);

    // Fill with data
    let cid1 = store.put(&[0u8; 100]).unwrap();
    track_storage(&mut cap, 100).unwrap();
    let _cid2 = store.put(&[1u8; 100]).unwrap();
    track_storage(&mut cap, 100).unwrap();

    // Cap is full — reject
    assert!(track_storage(&mut cap, 50).is_err());

    // GC one expired CID
    let expired = vec![cid1.to_string()];
    let freed = garbage_collect(&store, &mut cap, &expired);
    assert_eq!(freed, 100);
    assert_eq!(cap.used_bytes, 100);

    // Now we can add again
    assert!(track_storage(&mut cap, 50).is_ok());
    assert_eq!(cap.used_bytes, 150);
}

#[test]
fn gc_nonexistent_cid_frees_nothing() {
    let store = CidStore::new();
    let mut cap = make_cap(1000);
    track_storage(&mut cap, 500).unwrap();

    let freed = garbage_collect(&store, &mut cap, &["not-a-valid-cid".to_string()]);
    assert_eq!(freed, 0);
    assert_eq!(cap.used_bytes, 500);
}

#[test]
fn shard_residency_matching() {
    assert!(check_shard_residency("US", "US"));
    assert!(check_shard_residency("us", "US"));
    assert!(!check_shard_residency("DE", "US"));
    assert!(check_shard_residency("DE", "any"));
    assert!(check_shard_residency("JP", ""));
}

#[test]
fn classify_workload_clean_passes() {
    assert!(classify_workload("Train a neural network on CIFAR-10").is_ok());
    assert!(classify_workload("Protein folding simulation").is_ok());
}

#[test]
fn classify_workload_banned_keywords_rejected() {
    let result = classify_workload("Run nmap scan on target network");
    assert!(result.is_err());

    let result = classify_workload("Deploy ransomware payload");
    assert!(result.is_err());

    let result = classify_workload("Password cracking with hashcat");
    assert!(result.is_err());
}
