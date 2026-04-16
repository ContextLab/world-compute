//! Node struct and state machine per data-model §3.3.

use crate::credits::caliber::CaliberClass;
use crate::sandbox::SandboxCapability;
use crate::scheduler::ResourceEnvelope;
use crate::types::{PeerIdStr, Timestamp, TrustScore};
use crate::verification::trust_score::TrustTier;
use serde::{Deserialize, Serialize};

/// Node lifecycle states per data-model §3.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeState {
    Joining,
    Idle,
    Leased,
    Preempted,
    Quarantined,
    Offline,
}

/// A logical instance of the agent on a single machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub peer_id: PeerIdStr,
    pub state: NodeState,
    pub trust_tier: TrustTier,
    pub caliber_class: CaliberClass,
    pub trust_score: TrustScore,
    pub sandbox_capability: SandboxCapability,
    pub capacity: ResourceEnvelope,
    pub last_heartbeat: Timestamp,
}
