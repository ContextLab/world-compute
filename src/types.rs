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
/// We re-export the libp2p type for runtime use but use PeerIdStr in
/// serializable structs since libp2p::PeerId doesn't derive serde.
pub type PeerId = libp2p::PeerId;

/// String representation of a PeerId for use in serializable structs.
pub type PeerIdStr = String;

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

// ─── spec 005 additions (T010) ──────────────────────────────────────────

/// State of a single libp2p Relay v2 reservation held by this agent.
/// Transitions per data-model §A.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReservationStatus {
    /// Reservation request sent, awaiting response.
    Requesting,
    /// Reservation accepted and currently active.
    Active,
    /// Renewal request sent near expiry.
    Renewing,
    /// Reservation was dropped (relay reboot, connection loss). Must reacquire
    /// within 60 s per FR-006.
    Lost,
    /// Reservation request denied or timed out.
    Failed,
}

/// libp2p transport kind, used for dial-logging visibility (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportKind {
    /// Plain TCP + Noise handshake.
    Tcp,
    /// QUIC (UDP).
    Quic,
    /// WebSocket-over-TLS on port 443; spec 005 fallback for hostile firewalls.
    Wss,
    /// Connection via a libp2p relay-v2 circuit.
    Relay,
}

/// Outcome of a dial attempt (FR-004). Every non-success outcome is emitted
/// at `info` level or higher with full root-cause detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialOutcome {
    /// Connection established.
    Success,
    /// Dial timed out without upgrading.
    Timeout,
    /// Transport-layer error (TCP refused, QUIC unreachable, TLS handshake failure).
    TransportError(String),
    /// Remote peer explicitly denied the dial.
    Denied(String),
}

/// Safety tier for mesh-LLM / diffusion inference requests (FR-029).
/// Re-exported for convenience; the diffusion service uses this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SafetyTier {
    /// Content may be made public without further review.
    Public,
    /// Content is usable inside the organization / federation.
    Internal,
    /// Content is restricted; policy review required before exposure.
    Restricted,
}

/// Identifier for a specialized SSD-2-style diffusion expert (spec 005 US6).
/// Opaque UUID wrapper; experts are registered by ID and selected by the router.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExpertId(pub String);

impl ExpertId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ExpertId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExpertId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Denoising-step index within a diffusion inference request (0..denoising_steps).
/// Wrapper around u32 for type safety — avoids confusing token-step with denoising-step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DenoisingStep(pub u32);

impl fmt::Display for DenoisingStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "step-{}", self.0)
    }
}

#[cfg(test)]
mod spec_005_type_tests {
    use super::*;

    #[test]
    fn reservation_status_variants_distinct() {
        use ReservationStatus::*;
        let all = [Requesting, Active, Renewing, Lost, Failed];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j]);
            }
        }
    }

    #[test]
    fn transport_kind_variants_distinct() {
        use TransportKind::*;
        assert_ne!(Tcp, Quic);
        assert_ne!(Tcp, Wss);
        assert_ne!(Quic, Wss);
        assert_ne!(Wss, Relay);
    }

    #[test]
    fn expert_id_round_trip() {
        let a = ExpertId::new();
        let s = a.as_str().to_owned();
        let b = ExpertId::from_str(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn denoising_step_display() {
        assert_eq!(format!("{}", DenoisingStep(42)), "step-42");
    }
}
