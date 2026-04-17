//! Containment action execution — implements the incident response primitives.
//!
//! Per contracts/incident.md: authorized responders (OnCallResponder role)
//! can trigger containment actions. Each action produces an IncidentRecord.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::incident::audit::IncidentRecord;
use crate::incident::ContainmentAction;
use crate::types::Timestamp;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Shared state for containment enforcement.
///
/// The policy engine and scheduler query these sets to enforce containment.
#[derive(Debug, Clone)]
pub struct ContainmentState {
    /// Workload classes that are quarantined — policy engine rejects these.
    pub quarantined_classes: Arc<RwLock<HashSet<String>>>,
    /// Submitter IDs that are blocked — policy engine rejects jobs from these.
    pub blocked_submitters: Arc<RwLock<HashSet<String>>>,
    /// Artifact CIDs that have been revoked — removed from approved set.
    pub revoked_artifacts: Arc<RwLock<HashSet<String>>>,
    /// Host pools marked as draining — scheduler migrates workloads off.
    pub draining_pools: Arc<RwLock<HashSet<String>>>,
    /// Frozen host peer IDs.
    pub frozen_hosts: Arc<RwLock<HashSet<String>>>,
}

impl ContainmentState {
    pub fn new() -> Self {
        Self {
            quarantined_classes: Arc::new(RwLock::new(HashSet::new())),
            blocked_submitters: Arc::new(RwLock::new(HashSet::new())),
            revoked_artifacts: Arc::new(RwLock::new(HashSet::new())),
            draining_pools: Arc::new(RwLock::new(HashSet::new())),
            frozen_hosts: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if a workload class is quarantined.
    pub fn is_class_quarantined(&self, class: &str) -> bool {
        self.quarantined_classes.read().unwrap().contains(class)
    }

    /// Check if a submitter is blocked.
    pub fn is_submitter_blocked(&self, submitter: &str) -> bool {
        self.blocked_submitters.read().unwrap().contains(submitter)
    }

    /// Check if an artifact CID has been revoked.
    pub fn is_artifact_revoked(&self, cid: &str) -> bool {
        self.revoked_artifacts.read().unwrap().contains(cid)
    }

    /// Check if a pool is draining.
    pub fn is_pool_draining(&self, pool: &str) -> bool {
        self.draining_pools.read().unwrap().contains(pool)
    }

    /// Check if a host is frozen.
    pub fn is_host_frozen(&self, host: &str) -> bool {
        self.frozen_hosts.read().unwrap().contains(host)
    }
}

impl Default for ContainmentState {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute freeze on a list of sandbox PIDs by sending SIGSTOP.
///
/// On Unix, iterates the PID list and sends SIGSTOP to each via `nix::sys::signal`.
/// Returns the count of successfully stopped processes.
pub fn execute_freeze_host(pids: &[u32]) -> Result<usize, WcError> {
    let mut stopped = 0usize;

    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;

        for &pid in pids {
            match signal::kill(Pid::from_raw(pid as i32), Signal::SIGSTOP) {
                Ok(()) => {
                    stopped += 1;
                }
                Err(e) => {
                    tracing::warn!(pid, error = %e, "Failed to send SIGSTOP to process");
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = pids;
        tracing::warn!("Freeze not supported on this platform");
    }

    Ok(stopped)
}

/// Execute quarantine: add workload class to the quarantine rejection set.
pub fn execute_quarantine_class(state: &ContainmentState, class: &str) {
    state.quarantined_classes.write().unwrap().insert(class.to_string());
    tracing::info!(class, "Workload class quarantined");
}

/// Execute block submitter: add submitter ID to the ban set.
/// Returns the count of in-flight jobs that would be cancelled (estimated).
pub fn execute_block_submitter(
    state: &ContainmentState,
    submitter_id: &str,
    in_flight_count: usize,
) -> usize {
    state.blocked_submitters.write().unwrap().insert(submitter_id.to_string());
    tracing::info!(submitter_id, in_flight_count, "Submitter blocked");
    in_flight_count
}

/// Execute revoke artifact: remove CID from the approved set and track as revoked.
/// Returns count of affected jobs (estimated, passed in by caller).
pub fn execute_revoke_artifact(
    state: &ContainmentState,
    cid_str: &str,
    affected_jobs: usize,
) -> usize {
    state.revoked_artifacts.write().unwrap().insert(cid_str.to_string());
    tracing::info!(cid = cid_str, affected_jobs, "Artifact revoked");
    affected_jobs
}

/// Execute drain pool: mark a pool as draining.
/// Returns count of workloads that need to be migrated (estimated, passed in by caller).
pub fn execute_drain_pool(state: &ContainmentState, pool_id: &str, workload_count: usize) -> usize {
    state.draining_pools.write().unwrap().insert(pool_id.to_string());
    tracing::info!(pool_id, workload_count, "Pool marked as draining");
    workload_count
}

/// Execute a containment action, returning an audit record.
///
/// Caller must verify OnCallResponder role before calling this function.
pub fn execute_containment(
    action: ContainmentAction,
    target: &str,
    actor_peer_id: &str,
    actor_role: &str,
    justification: &str,
    incident_id: &str,
) -> WcResult<IncidentRecord> {
    // Verify caller has appropriate role
    if actor_role != "OnCallResponder" {
        return Err(WcError::new(
            ErrorCode::PermissionDenied,
            format!("Containment actions require OnCallResponder role, got '{actor_role}'"),
        ));
    }

    let record_id = format!("ir-{}-{}", Timestamp::now().0, &target[..8.min(target.len())]);

    let record = IncidentRecord::new(
        record_id,
        incident_id.to_string(),
        action,
        target.to_string(),
        actor_peer_id.to_string(),
        actor_role.to_string(),
        justification.to_string(),
    );

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_role_rejected() {
        let result = execute_containment(
            ContainmentAction::FreezeHost,
            "host-123",
            "peer-abc",
            "RegularUser",
            "suspicious activity",
            "incident-001",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn authorized_action_succeeds() {
        let record = execute_containment(
            ContainmentAction::FreezeHost,
            "host-123",
            "peer-abc",
            "OnCallResponder",
            "suspicious activity",
            "incident-001",
        )
        .unwrap();
        assert_eq!(record.action_type, ContainmentAction::FreezeHost);
        assert!(record.reversible);
        assert_eq!(record.target, "host-123");
    }

    #[test]
    fn revoke_artifact_not_reversible() {
        let record = execute_containment(
            ContainmentAction::RevokeArtifact,
            "bafybeig...",
            "peer-abc",
            "OnCallResponder",
            "compromised artifact",
            "incident-002",
        )
        .unwrap();
        assert!(!record.reversible);
    }

    #[test]
    fn freeze_empty_pids_returns_zero() {
        let count = execute_freeze_host(&[]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn quarantine_adds_class() {
        let state = ContainmentState::new();
        assert!(!state.is_class_quarantined("crypto-mining"));
        execute_quarantine_class(&state, "crypto-mining");
        assert!(state.is_class_quarantined("crypto-mining"));
        assert!(!state.is_class_quarantined("ml-training"));
    }

    #[test]
    fn block_submitter_adds_to_ban_set() {
        let state = ContainmentState::new();
        assert!(!state.is_submitter_blocked("evil-user"));
        let cancelled = execute_block_submitter(&state, "evil-user", 5);
        assert_eq!(cancelled, 5);
        assert!(state.is_submitter_blocked("evil-user"));
        assert!(!state.is_submitter_blocked("good-user"));
    }

    #[test]
    fn revoke_artifact_tracks_cid() {
        let state = ContainmentState::new();
        assert!(!state.is_artifact_revoked("bafyabc123"));
        let affected = execute_revoke_artifact(&state, "bafyabc123", 3);
        assert_eq!(affected, 3);
        assert!(state.is_artifact_revoked("bafyabc123"));
    }

    #[test]
    fn drain_pool_marks_draining() {
        let state = ContainmentState::new();
        assert!(!state.is_pool_draining("pool-us-east-1"));
        let migrated = execute_drain_pool(&state, "pool-us-east-1", 10);
        assert_eq!(migrated, 10);
        assert!(state.is_pool_draining("pool-us-east-1"));
    }

    #[test]
    fn containment_state_default() {
        let state = ContainmentState::default();
        assert!(!state.is_class_quarantined("any"));
        assert!(!state.is_submitter_blocked("any"));
        assert!(!state.is_artifact_revoked("any"));
        assert!(!state.is_pool_draining("any"));
        assert!(!state.is_host_frozen("any"));
    }
}
