//! Threshold signing for ledger entries per FR-051 (T138).
//!
//! Uses `threshold_crypto` for t-of-n threshold BLS signatures over
//! ledger Merkle roots and entry batches.

use threshold_crypto::{PublicKeySet, SecretKeyShare, SignatureShare};

/// Generate a threshold key set: t-of-n where `threshold` signers are
/// required out of `total` key holders.
pub fn generate_threshold_keys(
    threshold: usize,
    total: usize,
) -> (PublicKeySet, Vec<SecretKeyShare>) {
    let mut rng = rand_04::thread_rng();
    let sk_set = threshold_crypto::SecretKeySet::random(threshold - 1, &mut rng);
    let pk_set = sk_set.public_keys();
    let shares: Vec<SecretKeyShare> = (0..total).map(|i| sk_set.secret_key_share(i)).collect();
    (pk_set, shares)
}

/// Sign a message with a single secret key share.
pub fn sign_share(share: &SecretKeyShare, message: &[u8]) -> SignatureShare {
    share.sign(message)
}

/// Combine threshold signature shares into a full signature.
/// Requires at least `threshold` valid shares.
pub fn combine_signatures(
    pk_set: &PublicKeySet,
    shares: &[(usize, SignatureShare)],
) -> Result<threshold_crypto::Signature, threshold_crypto::error::Error> {
    let share_refs: std::collections::BTreeMap<usize, &SignatureShare> =
        shares.iter().map(|(i, s)| (*i, s)).collect();
    pk_set.combine_signatures(share_refs)
}

/// Verify a combined threshold signature against the public key set.
pub fn verify_threshold_signature(
    pk_set: &PublicKeySet,
    message: &[u8],
    sig: &threshold_crypto::Signature,
) -> bool {
    pk_set.public_key().verify(sig, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_3_of_5_round_trip() {
        let (pk_set, shares) = generate_threshold_keys(3, 5);
        let message = b"merkle-root-hash-sentinel-for-threshold-test";

        // Sign with 3 out of 5 shares
        let sig_shares: Vec<(usize, SignatureShare)> = shares
            .iter()
            .enumerate()
            .take(3)
            .map(|(i, share)| (i, sign_share(share, message)))
            .collect();

        let combined = combine_signatures(&pk_set, &sig_shares).expect("combine should succeed");
        assert!(verify_threshold_signature(&pk_set, message, &combined));
    }

    #[test]
    fn insufficient_shares_fails() {
        let (pk_set, shares) = generate_threshold_keys(3, 5);
        let message = b"test-message";

        // Only 2 shares — below threshold of 3
        let sig_shares: Vec<(usize, SignatureShare)> = shares
            .iter()
            .enumerate()
            .take(2)
            .map(|(i, share)| (i, sign_share(share, message)))
            .collect();

        let result = combine_signatures(&pk_set, &sig_shares);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_message_fails_verification() {
        let (pk_set, shares) = generate_threshold_keys(3, 5);
        let message = b"correct-message";
        let wrong = b"wrong-message";

        let sig_shares: Vec<(usize, SignatureShare)> = shares
            .iter()
            .enumerate()
            .take(3)
            .map(|(i, share)| (i, sign_share(share, message)))
            .collect();

        let combined = combine_signatures(&pk_set, &sig_shares).expect("combine should succeed");
        assert!(!verify_threshold_signature(&pk_set, wrong, &combined));
    }
}
