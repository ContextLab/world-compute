//! IncidentRecord — immutable audit records for containment actions.
//!
//! Per FR-S061 and data-model.md: every containment action MUST be logged
//! with actor identity, timestamp, justification, and reversibility status.

use crate::incident::ContainmentAction;
use crate::types::{PeerIdStr, Timestamp};
use serde::{Deserialize, Serialize};

/// An immutable record of a containment action taken during incident response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentRecord {
    /// Unique identifier for this record.
    pub record_id: String,
    /// Groups related actions into one incident.
    pub incident_id: String,
    /// Type of action taken.
    pub action_type: ContainmentAction,
    /// What the action targets (host ID, workload class, submitter ID, artifact CID).
    pub target: String,
    /// Identity of the responder who took the action.
    pub actor_peer_id: PeerIdStr,
    /// Role under which the action was authorized.
    pub actor_role: String,
    /// Why the action was taken.
    pub justification: String,
    /// Whether the action can be undone.
    pub reversible: bool,
    /// If reversed, the record_id of the reversal action.
    pub reversed_by: Option<String>,
    /// When the action was taken.
    pub timestamp: Timestamp,
}

impl IncidentRecord {
    /// Create a new incident record for a containment action.
    pub fn new(
        record_id: String,
        incident_id: String,
        action_type: ContainmentAction,
        target: String,
        actor_peer_id: PeerIdStr,
        actor_role: String,
        justification: String,
    ) -> Self {
        Self {
            record_id,
            incident_id,
            reversible: action_type.is_reversible(),
            action_type,
            target,
            actor_peer_id,
            actor_role,
            justification,
            reversed_by: None,
            timestamp: Timestamp::now(),
        }
    }
}
