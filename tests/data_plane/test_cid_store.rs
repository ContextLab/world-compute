//! Integration tests for CID store operations (T104).

use worldcompute::data_plane::cid_store::{compute_cid, CidStore};

#[test]
fn put_get_round_trip() {
    let store = CidStore::new();
    let data = b"integration test data for cid store";
    let cid = store.put(data).unwrap();
    let retrieved = store.get(&cid).unwrap();
    assert_eq!(retrieved, data);
}

#[test]
fn has_existing_cid_returns_true() {
    let store = CidStore::new();
    let cid = store.put(b"exists").unwrap();
    assert!(store.has(&cid));
}

#[test]
fn has_missing_cid_returns_false() {
    let store = CidStore::new();
    let cid = compute_cid(b"never stored").unwrap();
    assert!(!store.has(&cid));
}

#[test]
fn compute_cid_deterministic() {
    let data = b"deterministic content";
    let cid1 = compute_cid(data).unwrap();
    let cid2 = compute_cid(data).unwrap();
    assert_eq!(cid1, cid2, "Same data must produce same CID");
}

#[test]
fn store_multiple_objects() {
    let store = CidStore::new();
    let cid1 = store.put(b"object one").unwrap();
    let cid2 = store.put(b"object two").unwrap();
    let cid3 = store.put(b"object three").unwrap();
    assert_ne!(cid1, cid2);
    assert_ne!(cid2, cid3);
    assert_eq!(store.len(), 3);
}
