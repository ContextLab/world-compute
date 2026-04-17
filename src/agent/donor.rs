//! Donor struct per data-model §3.2.

use crate::acceptable_use::{AcceptableUseClass, ShardCategory};
use crate::credits::caliber::CaliberClass;
use crate::types::{NcuAmount, PeerIdStr, Timestamp, TrustScore};
use serde::{Deserialize, Serialize};

/// A hardware donor — a person or operator who opts in to run the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Donor {
    pub donor_id: DonorId,
    pub peer_id: PeerIdStr,
    pub caliber_class: CaliberClass,
    pub credit_balance: NcuAmount,
    pub trust_score: TrustScore,
    pub consent_classes: Vec<AcceptableUseClass>,
    pub shard_allowlist: Vec<ShardCategory>,
    pub enrolled_at: Timestamp,
}

/// Strongly-typed donor ID with enforced format per FR-S072.
///
/// Format: "wc-donor-{hex_encoded_hash}" where hash is derived from
/// the donor's Ed25519 public key. This ensures uniqueness (one ID per key)
/// and a consistent, non-opaque format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DonorId(String);

impl DonorId {
    /// Create a DonorId from a peer's public key bytes.
    pub fn from_public_key(public_key: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(public_key);
        Self(format!("wc-donor-{}", hex::encode(&hash[..16])))
    }

    /// Validate an existing donor ID string.
    pub fn from_string(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();
        if !s.starts_with("wc-donor-") {
            return Err(format!("Invalid donor ID format: must start with 'wc-donor-', got '{s}'"));
        }
        let hex_part = &s["wc-donor-".len()..];
        if hex_part.len() != 32 {
            return Err(format!(
                "Invalid donor ID: hex part must be 32 chars, got {}",
                hex_part.len()
            ));
        }
        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid donor ID: hex part contains non-hex characters".into());
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DonorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donor_id_from_public_key_is_deterministic() {
        let key = [0xAA; 32];
        let id1 = DonorId::from_public_key(&key);
        let id2 = DonorId::from_public_key(&key);
        assert_eq!(id1, id2);
        assert!(id1.as_str().starts_with("wc-donor-"));
    }

    #[test]
    fn different_keys_different_ids() {
        let id1 = DonorId::from_public_key(&[0xAA; 32]);
        let id2 = DonorId::from_public_key(&[0xBB; 32]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn valid_donor_id_string_accepted() {
        let key = [0xCC; 32];
        let id = DonorId::from_public_key(&key);
        let parsed = DonorId::from_string(id.as_str()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn invalid_prefix_rejected() {
        assert!(DonorId::from_string("bad-prefix-abcdef1234567890abcdef1234567890").is_err());
    }

    #[test]
    fn wrong_length_rejected() {
        assert!(DonorId::from_string("wc-donor-abc").is_err());
    }
}
