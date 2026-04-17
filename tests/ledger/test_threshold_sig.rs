//! Integration tests for threshold signing (T138, T143).

use worldcompute::ledger::threshold_sig::{
    combine_signatures, generate_threshold_keys, sign_share, verify_threshold_signature,
};

#[test]
fn threshold_3_of_5_round_trip() {
    let (pk_set, shares) = generate_threshold_keys(3, 5);
    let message = b"ledger-merkle-root-20260416";

    // Collect 3 signature shares
    let sig_shares: Vec<(usize, _)> = shares
        .iter()
        .enumerate()
        .take(3)
        .map(|(i, share)| (i, sign_share(share, message)))
        .collect();

    let combined = combine_signatures(&pk_set, &sig_shares).expect("combine should succeed");
    assert!(
        verify_threshold_signature(&pk_set, message, &combined),
        "Threshold signature should verify"
    );
}

#[test]
fn any_3_of_5_shares_work() {
    let (pk_set, shares) = generate_threshold_keys(3, 5);
    let message = b"any-subset-test";

    // Use shares 1, 3, 4 (not the first three)
    let sig_shares: Vec<(usize, _)> = vec![
        (1, sign_share(&shares[1], message)),
        (3, sign_share(&shares[3], message)),
        (4, sign_share(&shares[4], message)),
    ];

    let combined = combine_signatures(&pk_set, &sig_shares).expect("combine should succeed");
    assert!(verify_threshold_signature(&pk_set, message, &combined));
}

#[test]
fn insufficient_shares_fails() {
    let (pk_set, shares) = generate_threshold_keys(3, 5);
    let message = b"not-enough-shares";

    let sig_shares: Vec<(usize, _)> = shares
        .iter()
        .enumerate()
        .take(2)
        .map(|(i, share)| (i, sign_share(share, message)))
        .collect();

    let result = combine_signatures(&pk_set, &sig_shares);
    assert!(result.is_err(), "2-of-5 should fail for threshold 3");
}

#[test]
fn wrong_message_fails_verification() {
    let (pk_set, shares) = generate_threshold_keys(3, 5);
    let message = b"signed-this";
    let wrong = b"not-this";

    let sig_shares: Vec<(usize, _)> = shares
        .iter()
        .enumerate()
        .take(3)
        .map(|(i, share)| (i, sign_share(share, message)))
        .collect();

    let combined = combine_signatures(&pk_set, &sig_shares).unwrap();
    assert!(
        !verify_threshold_signature(&pk_set, wrong, &combined),
        "Wrong message should not verify"
    );
}
