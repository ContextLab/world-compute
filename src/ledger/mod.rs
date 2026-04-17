//! Ledger module — append-only Merkle-chained tamper-evident record.
//!
//! NOT a blockchain. CRDT-replicated, threshold-signed, anchored to
//! Sigstore Rekor every 10 minutes per FR-051.

pub mod crdt;
pub mod entry;
pub mod threshold_sig;
pub mod transparency;

pub use entry::{LedgerEntry, LedgerEntryType, LedgerShard, MerkleRoot};
