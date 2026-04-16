//! Ed25519 key generation and PeerId derivation per T018.

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::Path;

/// Generate a new Ed25519 signing key.
pub fn generate_signing_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

/// Load an existing key from file or generate and persist a new one.
pub fn load_or_create_key(path: &Path) -> Result<SigningKey, crate::error::WcError> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| crate::error::WcError::new(
                crate::error::ErrorCode::Internal,
                "Invalid key file length",
            ))?;
        Ok(SigningKey::from_bytes(&key_bytes))
    } else {
        let key = generate_signing_key();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, key.to_bytes())?;
        Ok(key)
    }
}

/// Derive a libp2p PeerId from an Ed25519 signing key.
pub fn peer_id_from_key(key: &SigningKey) -> libp2p::PeerId {
    let public = key.verifying_key();
    let libp2p_key = libp2p::identity::ed25519::PublicKey::try_from_bytes(
        public.as_bytes(),
    ).expect("valid ed25519 public key");
    let libp2p_pubkey = libp2p::identity::PublicKey::from(libp2p_key);
    libp2p::PeerId::from_public_key(&libp2p_pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_generation_produces_valid_peer_id() {
        let key = generate_signing_key();
        let pid = peer_id_from_key(&key);
        assert!(!pid.to_string().is_empty());
    }
}
