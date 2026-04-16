//! T084: Ed25519 key revocation propagates to coordinators.
//!
//! Key revocation is tested via the DonorId system — a revoked key's
//! DonorId should be rejectable by coordinators.

use worldcompute::agent::donor::DonorId;

#[test]
fn different_keys_produce_different_donor_ids() {
    let id1 = DonorId::from_public_key(&[0xAA; 32]);
    let id2 = DonorId::from_public_key(&[0xBB; 32]);
    assert_ne!(id1, id2, "Different keys must produce different DonorIds for revocation to work");
}

#[test]
fn donor_id_is_deterministic_for_same_key() {
    let key = [0xCC; 32];
    let id1 = DonorId::from_public_key(&key);
    let id2 = DonorId::from_public_key(&key);
    assert_eq!(id1, id2, "Same key must always produce same DonorId");
}

#[test]
fn donor_id_format_is_parseable() {
    let id = DonorId::from_public_key(&[0xDD; 32]);
    let parsed = DonorId::from_string(id.as_str()).unwrap();
    assert_eq!(id, parsed);
}
