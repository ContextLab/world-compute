//! Incident response — containment actions with full audit trails.
//!
//! Per FR-S060: supports FreezeHost, QuarantineWorkloadClass, BlockSubmitter,
//! RevokeArtifact, and DrainHostPool containment actions. All actions produce
//! immutable IncidentRecords per FR-S061.

pub mod audit;
pub mod containment;

use serde::{Deserialize, Serialize};

/// Types of containment actions available during incident response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainmentAction {
    /// Remove host from scheduling pool; no new jobs dispatched.
    FreezeHost,
    /// Policy engine rejects all jobs of this workload class.
    QuarantineWorkloadClass,
    /// Policy engine rejects all jobs from this submitter.
    BlockSubmitter,
    /// Artifact removed from approved registry.
    RevokeArtifact,
    /// Checkpoint all running jobs on pool, migrate, remove from scheduling.
    DrainHostPool,
    /// Reversal actions.
    LiftFreeze,
    LiftQuarantine,
    UnblockSubmitter,
}

impl ContainmentAction {
    /// Whether this action type is reversible.
    pub fn is_reversible(self) -> bool {
        match self {
            Self::FreezeHost
            | Self::QuarantineWorkloadClass
            | Self::BlockSubmitter
            | Self::DrainHostPool => true,
            Self::RevokeArtifact => false, // re-approval required
            Self::LiftFreeze | Self::LiftQuarantine | Self::UnblockSubmitter => true,
        }
    }
}
