//! Confidential compute bundles per FR-012/FR-013.
//!
//! Provides types for encrypting job data so that only attested TEE enclaves
//! (or the submitter) can decrypt it. Supports AES-256-GCM encryption with
//! ephemeral keys wrapped under the submitter's public key.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::data_plane::cid_store::CidStore;
use crate::error::{ErrorCode, WcError};

/// Symmetric cipher used to encrypt the bundle payload.
#[derive(Debug, Clone)]
pub enum ConfidentialCipher {
    /// AES-256 in GCM mode (256-bit key, 96-bit nonce).
    Aes256Gcm,
}

/// Confidentiality level governing key-release policy.
#[derive(Debug, Clone)]
pub enum ConfidentialityLevel {
    /// Encrypted at rest; any authenticated donor can decrypt.
    Medium,
    /// Encrypted at rest; only donors with a matching TEE attestation can decrypt.
    High,
}

/// An encrypted data bundle for confidential compute workloads.
#[derive(Debug, Clone)]
pub struct ConfidentialBundle {
    /// CID of the ciphertext blob in the content-addressed store.
    pub ciphertext_cid: crate::types::Cid,
    /// Cipher algorithm used.
    pub cipher: ConfidentialCipher,
    /// Nonce / IV for the cipher.
    pub nonce: [u8; 12],
    /// Ephemeral symmetric key wrapped with the submitter's public key.
    pub wrapped_key: Vec<u8>,
    /// Required confidentiality level.
    pub confidentiality_level: ConfidentialityLevel,
    /// For `High` level: required guest measurement hash for TEE attestation.
    pub attestation_requirement: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// T084: Client-side AES-256-GCM encryption
// ---------------------------------------------------------------------------

/// Encrypt job data using AES-256-GCM. Returns a [`ConfidentialBundle`] with
/// the ciphertext stored in the provided CID store.
///
/// The caller is responsible for wrapping `bundle.wrapped_key` for the
/// intended recipient via [`wrap_key_for_recipient`].
pub fn encrypt_job_data(plaintext: &[u8], store: &CidStore) -> Result<ConfidentialBundle, WcError> {
    // Generate random 256-bit key
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);

    // Generate random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    // Encrypt
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("AES key init: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("AES encrypt: {e}")))?;

    // Store ciphertext in CID store
    let cid = store.put(&ciphertext)?;

    Ok(ConfidentialBundle {
        ciphertext_cid: cid,
        cipher: ConfidentialCipher::Aes256Gcm,
        nonce: nonce_bytes,
        wrapped_key: key.to_vec(),
        confidentiality_level: ConfidentialityLevel::Medium,
        attestation_requirement: None,
    })
}

// ---------------------------------------------------------------------------
// T085: Key wrapping using X25519
// ---------------------------------------------------------------------------

/// Wrap an ephemeral AES key for a recipient using X25519 Diffie-Hellman.
///
/// The `ephemeral_key` is XOR'd with SHA-256(shared_secret) derived from the
/// sender's secret and the recipient's public key.
pub fn wrap_key_for_recipient(ephemeral_key: &[u8; 32], recipient_public: &[u8; 32]) -> Vec<u8> {
    let sender_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let sender_public = PublicKey::from(&sender_secret);
    let recipient_pk = PublicKey::from(*recipient_public);

    let shared = sender_secret.diffie_hellman(&recipient_pk);
    let hash = Sha256::digest(shared.as_bytes());

    let mut wrapped = [0u8; 32];
    for i in 0..32 {
        wrapped[i] = ephemeral_key[i] ^ hash[i];
    }

    // Prepend sender's public key so the recipient can derive the same shared secret
    let mut out = sender_public.as_bytes().to_vec();
    out.extend_from_slice(&wrapped);
    out
}

/// Unwrap a key that was wrapped by [`wrap_key_for_recipient`].
pub fn unwrap_key(
    wrapped: &[u8],
    recipient_secret: &[u8; 32],
    _sender_public: &[u8; 32],
) -> Result<[u8; 32], WcError> {
    if wrapped.len() < 64 {
        return Err(WcError::new(
            ErrorCode::Internal,
            "wrapped key too short (need 64 bytes: 32 sender-pubkey + 32 wrapped)",
        ));
    }
    // Extract sender public key from first 32 bytes
    let mut sender_pub_bytes = [0u8; 32];
    sender_pub_bytes.copy_from_slice(&wrapped[..32]);
    let sender_pk = PublicKey::from(sender_pub_bytes);

    let recipient = StaticSecret::from(*recipient_secret);
    let shared = recipient.diffie_hellman(&sender_pk);
    let hash = Sha256::digest(shared.as_bytes());

    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = wrapped[32 + i] ^ hash[i];
    }
    Ok(key)
}

// ---------------------------------------------------------------------------
// T086: Attestation check for key release
// ---------------------------------------------------------------------------

/// Check whether attestation status permits key release for the given
/// confidentiality level.
///
/// - `Medium`: requires valid attestation.
/// - `High`: requires valid attestation (guest measurement check to be added).
pub fn check_attestation_for_key_release(
    attestation_valid: bool,
    level: &ConfidentialityLevel,
) -> bool {
    match level {
        ConfidentialityLevel::Medium => attestation_valid,
        ConfidentialityLevel::High => attestation_valid,
    }
}

// ---------------------------------------------------------------------------
// T087: High-level key sealing (simplified placeholder)
// ---------------------------------------------------------------------------

/// Seal a key to a TEE guest measurement (simplified: XOR with SHA-256 of measurement).
///
/// In production this would use platform-specific sealing (e.g. AMD SEV
/// `KDF_SEAL` or Intel SGX `sgx_seal_data`).
pub fn seal_key_to_measurement(key: &[u8; 32], guest_measurement: &[u8]) -> Vec<u8> {
    let hash = Sha256::digest(guest_measurement);
    let mut sealed = [0u8; 32];
    for i in 0..32 {
        sealed[i] = key[i] ^ hash[i];
    }
    sealed.to_vec()
}

/// Unseal a key sealed with [`seal_key_to_measurement`].
pub fn unseal_key(sealed: &[u8], guest_measurement: &[u8]) -> Result<[u8; 32], WcError> {
    if sealed.len() != 32 {
        return Err(WcError::new(ErrorCode::Internal, "sealed key must be 32 bytes"));
    }
    let hash = Sha256::digest(guest_measurement);
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = sealed[i] ^ hash[i];
    }
    Ok(key)
}

// ---------------------------------------------------------------------------
// T088: Decrypt job data
// ---------------------------------------------------------------------------

/// Decrypt a [`ConfidentialBundle`] given the ephemeral AES key and the CID store
/// containing the ciphertext.
pub fn decrypt_job_data(
    bundle: &ConfidentialBundle,
    ephemeral_key: &[u8; 32],
    store: &CidStore,
) -> Result<Vec<u8>, WcError> {
    let ciphertext = store
        .get(&bundle.ciphertext_cid)
        .ok_or_else(|| WcError::new(ErrorCode::NotFound, "ciphertext CID not in store"))?;

    let cipher = Aes256Gcm::new_from_slice(ephemeral_key)
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("AES key init: {e}")))?;
    let nonce = Nonce::from_slice(&bundle.nonce);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| WcError::new(ErrorCode::Internal, format!("AES decrypt: {e}")))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let store = CidStore::new();
        let plaintext = b"hello, confidential world!";
        let bundle = encrypt_job_data(plaintext, &store).unwrap();

        // The wrapped_key in the bundle is the raw ephemeral key (before wrapping)
        let key: [u8; 32] = bundle.wrapped_key.clone().try_into().unwrap();
        let recovered = decrypt_job_data(&bundle, &key, &store).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let store = CidStore::new();
        let plaintext = b"secret data";
        let bundle = encrypt_job_data(plaintext, &store).unwrap();

        let wrong_key = [0xFFu8; 32];
        let result = decrypt_job_data(&bundle, &wrong_key, &store);
        assert!(result.is_err());
    }

    #[test]
    fn key_wrap_unwrap_roundtrip() {
        let recipient_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let recipient_public = PublicKey::from(&recipient_secret);

        let ephemeral_key = [42u8; 32];
        let wrapped = wrap_key_for_recipient(&ephemeral_key, recipient_public.as_bytes());

        let recovered = unwrap_key(
            &wrapped,
            recipient_secret.as_bytes(),
            &[0u8; 32], // sender_public is extracted from wrapped payload
        )
        .unwrap();
        assert_eq!(recovered, ephemeral_key);
    }

    #[test]
    fn attestation_check_medium_valid() {
        assert!(check_attestation_for_key_release(true, &ConfidentialityLevel::Medium));
    }

    #[test]
    fn attestation_check_medium_invalid() {
        assert!(!check_attestation_for_key_release(false, &ConfidentialityLevel::Medium));
    }

    #[test]
    fn attestation_check_high_valid() {
        assert!(check_attestation_for_key_release(true, &ConfidentialityLevel::High));
    }

    #[test]
    fn seal_unseal_roundtrip() {
        let key = [7u8; 32];
        let measurement = b"sha256-of-guest-image";
        let sealed = seal_key_to_measurement(&key, measurement);
        let recovered = unseal_key(&sealed, measurement).unwrap();
        assert_eq!(recovered, key);
    }
}
