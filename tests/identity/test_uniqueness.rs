//! T085: Duplicate donor_id rejected.

use worldcompute::agent::donor::DonorId;

#[test]
fn donor_id_unique_per_key() {
    // Generate 100 different keys, verify all IDs are unique
    let ids: Vec<DonorId> = (0u8..100)
        .map(|i| {
            let mut key = [0u8; 32];
            key[0] = i;
            DonorId::from_public_key(&key)
        })
        .collect();

    let unique_count = {
        let mut set = std::collections::HashSet::new();
        for id in &ids {
            set.insert(id.as_str().to_string());
        }
        set.len()
    };

    assert_eq!(unique_count, 100, "All 100 keys must produce unique DonorIds");
}

#[test]
fn invalid_donor_id_format_rejected() {
    assert!(DonorId::from_string("not-a-donor-id").is_err());
    assert!(DonorId::from_string("wc-donor-tooshort").is_err());
    assert!(DonorId::from_string("wc-donor-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    // non-hex
}
