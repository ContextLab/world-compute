//! Core type aliases and newtypes used across all modules.
//!
//! Per data-model §4 Type Reference Appendix.

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Content identifier (CIDv1 with SHA-256).
/// Wraps the `cid` crate's type for domain clarity.
pub type Cid = cid::Cid;

/// Peer identity derived from Ed25519 public key (libp2p PeerId).
pub type PeerId = libp2p::PeerId;

/// Normalized Compute Unit amount in micro-NCU (1 NCU = 1_000_000 micro-NCU).
/// Using u64 gives a range of ~18.4 billion NCU, sufficient for planetary scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NcuAmount(pub u64);

impl NcuAmount {
    pub const ZERO: Self = Self(0);

    pub fn as_micro_ncu(self) -> u64 {
        self.0
    }

    pub fn from_ncu(ncu: f64) -> Self {
        Self((ncu * 1_000_000.0) as u64)
    }

    pub fn as_ncu(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl fmt::Display for NcuAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} NCU", self.as_ncu())
    }
}

/// Timestamp as microseconds since Unix epoch (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch");
        Self(dur.as_micros() as u64)
    }
}

/// Duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DurationMs(pub u64);

/// Trust Score as a fixed-point value in [0, 10_000] representing [0.0, 1.0].
/// 10_000 = 1.0, 5_000 = 0.5, etc. Avoids floating-point non-determinism in
/// consensus-critical paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TrustScore(pub u16);

impl TrustScore {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(10_000);
    pub const NEW_NODE_CAP: Self = Self(5_000); // 0.5 cap for first 7 days

    pub fn as_f64(self) -> f64 {
        self.0 as f64 / 10_000.0
    }

    pub fn from_f64(v: f64) -> Self {
        Self((v.clamp(0.0, 1.0) * 10_000.0) as u16)
    }
}

impl fmt::Display for TrustScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.as_f64())
    }
}

/// Bundle of threshold signatures from coordinators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureBundle {
    /// Coordinator IDs that contributed signatures
    pub signer_ids: Vec<String>,
    /// The aggregated threshold signature bytes
    pub signature: Vec<u8>,
    /// t-of-n threshold parameters
    pub threshold: u32,
    pub total: u32,
}

/// Attestation quote from TPM, SEV-SNP, TDX, Apple SE, or soft attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationQuote {
    pub quote_type: AttestationType,
    pub quote_bytes: Vec<u8>,
    pub platform_info: String,
}

/// Type of hardware attestation available on the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttestationType {
    /// TPM 2.0 PCR quote (x86)
    Tpm2,
    /// AMD SEV-SNP attestation report
    SevSnp,
    /// Intel TDX quote
    Tdx,
    /// Apple Secure Enclave signing
    AppleSecureEnclave,
    /// Software-only attestation (WASM / low-trust tier)
    Soft,
}

/// Ed25519 public key for identity verification.
pub type PublicKey = VerifyingKey;
