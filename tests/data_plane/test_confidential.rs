//! Integration tests for confidential compute (T084–T089).

use worldcompute::data_plane::cid_store::CidStore;
use worldcompute::data_plane::confidential::{
    check_attestation_for_key_release, decrypt_job_data, encrypt_job_data, seal_key_to_measurement,
    unseal_key, unwrap_key, wrap_key_for_recipient, ConfidentialityLevel,
};

use x25519_dalek::{PublicKey, StaticSecret};

#[test]
fn encrypt_decrypt_roundtrip() {
    let store = CidStore::new();
    let plaintext = b"integration test: confidential job payload";
    let bundle = encrypt_job_data(plaintext, &store).unwrap();

    let key: [u8; 32] = bundle.wrapped_key.clone().try_into().unwrap();
    let recovered = decrypt_job_data(&bundle, &key, &store).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn decrypt_wrong_key_fails() {
    let store = CidStore::new();
    let plaintext = b"secret payload";
    let bundle = encrypt_job_data(plaintext, &store).unwrap();

    let wrong_key = [0xAAu8; 32];
    assert!(decrypt_job_data(&bundle, &wrong_key, &store).is_err());
}

#[test]
fn key_wrap_unwrap_roundtrip() {
    let recipient_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let recipient_public = PublicKey::from(&recipient_secret);

    let ephemeral_key = [0xBBu8; 32];
    let wrapped = wrap_key_for_recipient(&ephemeral_key, recipient_public.as_bytes());

    let recovered = unwrap_key(&wrapped, recipient_secret.as_bytes(), &[0u8; 32]).unwrap();
    assert_eq!(recovered, ephemeral_key);
}

#[test]
fn attestation_valid_medium_allowed() {
    assert!(check_attestation_for_key_release(true, &ConfidentialityLevel::Medium));
}

#[test]
fn attestation_invalid_medium_denied() {
    assert!(!check_attestation_for_key_release(false, &ConfidentialityLevel::Medium));
}

#[test]
fn seal_unseal_key_roundtrip() {
    let key = [0xCCu8; 32];
    let measurement = b"guest-measurement-hash-abc";
    let sealed = seal_key_to_measurement(&key, measurement);
    let recovered = unseal_key(&sealed, measurement).unwrap();
    assert_eq!(recovered, key);
}
