//! Donor struct per data-model §3.2.

use crate::acceptable_use::{AcceptableUseClass, ShardCategory};
use crate::credits::caliber::CaliberClass;
use crate::types::{NcuAmount, PeerIdStr, Timestamp, TrustScore};
use serde::{Deserialize, Serialize};

/// A hardware donor — a person or operator who opts in to run the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Donor {
    pub donor_id: String,
    pub peer_id: PeerIdStr,
    pub caliber_class: CaliberClass,
    pub credit_balance: NcuAmount,
    pub trust_score: TrustScore,
    pub consent_classes: Vec<AcceptableUseClass>,
    pub shard_allowlist: Vec<ShardCategory>,
    pub enrolled_at: Timestamp,
}
