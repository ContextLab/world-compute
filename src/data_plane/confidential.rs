//! Confidential compute bundles per FR-012/FR-013.
//!
//! Provides types for encrypting job data so that only attested TEE enclaves
//! (or the submitter) can decrypt it. Supports AES-256-GCM encryption with
//! ephemeral keys wrapped under the submitter's public key.

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
