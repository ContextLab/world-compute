//! Ledger entry types per data-model §3.12.

use crate::types::{Cid, SignatureBundle, Timestamp};
use serde::{Deserialize, Serialize};

/// Type of ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedgerEntryType {
    CreditEarn,
    CreditSpend,
    CreditDecay,
    CreditRefund,
    GovernanceRecord,
    AuditRecord,
}

/// A single entry in the append-only Merkle-chained ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Content-addressed ID of this entry.
    pub entry_cid: Cid,
    /// CID of the previous entry in this shard's chain.
    pub prev_cid: Option<Cid>,
    /// Sequence number within the shard.
    pub sequence: u64,
    /// Type of this entry.
    pub entry_type: LedgerEntryType,
    /// Timestamp of creation.
    pub timestamp: Timestamp,
    /// The donor or submitter this entry pertains to.
    pub subject_id: String,
    /// NCU amount (positive for earn/refund, negative for spend/decay).
    pub ncu_delta: i64,
    /// Opaque payload (CBOR-encoded details specific to entry type).
    pub payload: Vec<u8>,
    /// Threshold signature from coordinators.
    pub signature: SignatureBundle,
}

/// Per-coordinator chain head tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerShard {
    pub shard_id: String,
    pub coordinator_id: String,
    pub head_cid: Cid,
    pub head_sequence: u64,
    pub head_timestamp: Timestamp,
}

/// Cross-shard Merkle root checkpoint, anchored to Sigstore Rekor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRoot {
    pub root_hash: Vec<u8>,
    pub height: u64,
    pub timestamp: Timestamp,
    pub shard_heads: Vec<LedgerShard>,
    pub coordinator_signature: SignatureBundle,
    /// Sigstore Rekor entry ID for external anchoring.
    pub rekor_entry_id: Option<String>,
}
