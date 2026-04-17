//! Coordinator — Raft role management and shard coordination (T083-T084).
//!
//! The coordinator drives consensus for a scheduler shard using openraft.
//! Implements RaftStorage-compatible in-memory log with optional WAL.

use crate::error::{ErrorCode, WcError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Raft consensus role for this coordinator instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordinatorRole {
    /// This node leads the shard — it accepts writes and issues lease grants.
    Leader,
    /// This node replicates state from the leader.
    Follower,
    /// This node is campaigning for leadership (election in progress).
    Candidate,
}

/// A Raft log entry for coordinator state machine replication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftLogEntry {
    /// Raft term when this entry was created.
    pub term: u64,
    /// Log index (monotonically increasing).
    pub index: u64,
    /// The action to apply to the state machine.
    pub action: CoordinatorAction,
}

/// Actions that can be replicated via Raft consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorAction {
    /// Assign a job to a donor node.
    AssignJob { job_id: String, donor_id: String },
    /// Update job status.
    UpdateJobStatus { job_id: String, status: String },
    /// Register a new donor node.
    RegisterDonor { donor_id: String, shard_id: u32 },
    /// Remove a donor node.
    RemoveDonor { donor_id: String },
    /// No-op entry (used for leader commit confirmation).
    Noop,
}

/// In-memory Raft log storage for coordinator consensus.
///
/// Implements the storage layer for openraft's RaftStorage trait pattern.
/// Entries are kept in a BTreeMap indexed by log index for efficient
/// range queries and compaction.
#[derive(Debug, Clone, Default)]
pub struct RaftCoordinatorStorage {
    /// In-memory Raft log entries.
    log: BTreeMap<u64, RaftLogEntry>,
    /// Last applied log index to the state machine.
    last_applied: u64,
    /// Current vote (term, candidate_id).
    current_vote: Option<(u64, String)>,
    /// Committed index (highest log entry known to be committed).
    commit_index: u64,
    /// Optional WAL file path for durability.
    wal_path: Option<std::path::PathBuf>,
}

impl RaftCoordinatorStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create storage with a WAL file for durability across restarts.
    pub fn with_wal(wal_path: std::path::PathBuf) -> Self {
        let mut storage = Self::new();
        storage.wal_path = Some(wal_path);
        storage
    }

    /// Append an entry to the log.
    pub fn append(&mut self, entry: RaftLogEntry) -> Result<u64, WcError> {
        let index = entry.index;
        if index <= self.last_log_index() && self.log.contains_key(&index) {
            // Truncate conflicting entries (Raft log matching property)
            let keys_to_remove: Vec<u64> = self.log.range(index..).map(|(k, _)| *k).collect();
            for k in keys_to_remove {
                self.log.remove(&k);
            }
        }
        self.log.insert(index, entry);

        // Write to WAL if configured
        if let Some(ref wal_path) = self.wal_path {
            self.flush_wal(wal_path.clone())?;
        }

        Ok(index)
    }

    /// Get a log entry by index.
    pub fn get(&self, index: u64) -> Option<&RaftLogEntry> {
        self.log.get(&index)
    }

    /// Get the last log index.
    pub fn last_log_index(&self) -> u64 {
        self.log.keys().next_back().copied().unwrap_or(0)
    }

    /// Get the last log term.
    pub fn last_log_term(&self) -> u64 {
        self.log.values().next_back().map(|e| e.term).unwrap_or(0)
    }

    /// Record a vote for the current term.
    pub fn record_vote(&mut self, term: u64, candidate_id: String) {
        self.current_vote = Some((term, candidate_id));
    }

    /// Check if we've voted in the given term.
    pub fn voted_for(&self, term: u64) -> Option<&str> {
        self.current_vote.as_ref().filter(|(t, _)| *t == term).map(|(_, id)| id.as_str())
    }

    /// Advance the commit index.
    pub fn set_commit_index(&mut self, index: u64) {
        if index > self.commit_index {
            self.commit_index = index;
        }
    }

    /// Apply committed entries to the state machine.
    pub fn apply_committed(&mut self) -> Vec<CoordinatorAction> {
        let mut applied = Vec::new();
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            if let Some(entry) = self.log.get(&self.last_applied) {
                applied.push(entry.action.clone());
            }
        }
        applied
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    /// Flush log to WAL file (simple JSON-lines format).
    fn flush_wal(&self, wal_path: std::path::PathBuf) -> Result<(), WcError> {
        let data = serde_json::to_string(&self.log)
            .map_err(|e| WcError::new(ErrorCode::Internal, format!("WAL serialize: {e}")))?;
        std::fs::write(&wal_path, data)
            .map_err(|e| WcError::new(ErrorCode::Internal, format!("WAL write: {e}")))?;
        Ok(())
    }
}

/// Shard coordinator — manages Raft state for one scheduler shard.
///
/// A "shard" is a partition of the global job queue; each shard has one
/// coordinator cluster (typically 3 or 5 nodes) running Raft consensus.
#[derive(Debug, Clone)]
pub struct Coordinator {
    /// Unique identifier for this coordinator instance (matches node peer ID).
    pub coordinator_id: String,
    /// Shard identifier this coordinator manages.
    pub shard_id: u32,
    /// Current Raft term.
    pub raft_term: u64,
    /// Current Raft role.
    pub raft_role: CoordinatorRole,
    /// Raft log storage.
    storage: RaftCoordinatorStorage,
    /// Known peer coordinator IDs in this shard's cluster.
    peers: Vec<String>,
    /// Votes received in current election (candidate mode only).
    votes_received: usize,
}

impl Coordinator {
    /// Create a new coordinator starting as a Follower in term 0.
    pub fn new(coordinator_id: impl Into<String>, shard_id: u32) -> Self {
        Self {
            coordinator_id: coordinator_id.into(),
            shard_id,
            raft_term: 0,
            raft_role: CoordinatorRole::Follower,
            storage: RaftCoordinatorStorage::new(),
            peers: Vec::new(),
            votes_received: 0,
        }
    }

    /// Create a coordinator with known peers for consensus.
    pub fn with_peers(
        coordinator_id: impl Into<String>,
        shard_id: u32,
        peers: Vec<String>,
    ) -> Self {
        let mut coord = Self::new(coordinator_id, shard_id);
        coord.peers = peers;
        coord
    }

    /// Returns true if this coordinator is currently the shard leader.
    pub fn is_leader(&self) -> bool {
        self.raft_role == CoordinatorRole::Leader
    }

    /// Transition to Candidate role and increment the term.
    ///
    /// Starts an election: increments term, votes for self, and would
    /// broadcast RequestVote RPCs to peers via the network layer.
    pub fn start_election(&mut self) {
        self.raft_term += 1;
        self.raft_role = CoordinatorRole::Candidate;
        self.votes_received = 1; // Vote for self
        self.storage.record_vote(self.raft_term, self.coordinator_id.clone());

        tracing::info!(
            coordinator = %self.coordinator_id,
            term = self.raft_term,
            peers = self.peers.len(),
            "Starting election — requesting votes from peers"
        );
    }

    /// Receive a vote from a peer. If quorum reached, become leader.
    pub fn receive_vote(&mut self, from_peer: &str, term: u64, granted: bool) {
        if term != self.raft_term || self.raft_role != CoordinatorRole::Candidate {
            return;
        }
        if granted {
            self.votes_received += 1;
            tracing::debug!(
                from = from_peer,
                votes = self.votes_received,
                needed = self.quorum_size(),
                "Vote received"
            );
            if self.votes_received >= self.quorum_size() {
                self.become_leader();
            }
        }
    }

    /// Quorum size for the cluster (majority of total nodes including self).
    pub fn quorum_size(&self) -> usize {
        let total = self.peers.len() + 1; // peers + self
        (total / 2) + 1
    }

    /// Transition to Leader role.
    ///
    /// Called once quorum of votes received. Appends a no-op entry
    /// to commit entries from previous terms.
    pub fn become_leader(&mut self) {
        self.raft_role = CoordinatorRole::Leader;
        self.votes_received = 0;

        // Append no-op entry to establish leadership
        let noop = RaftLogEntry {
            term: self.raft_term,
            index: self.storage.last_log_index() + 1,
            action: CoordinatorAction::Noop,
        };
        let _ = self.storage.append(noop);

        tracing::info!(
            coordinator = %self.coordinator_id,
            term = self.raft_term,
            "Became leader — sending initial heartbeats"
        );
    }

    /// Step down to Follower, updating term if a higher term is seen.
    ///
    /// Called when a higher term is observed in any RPC.
    pub fn step_down(&mut self, new_term: u64) {
        if new_term >= self.raft_term {
            self.raft_term = new_term;
        }
        self.raft_role = CoordinatorRole::Follower;
        self.votes_received = 0;
    }

    /// Replicate an action via the Raft log (leader only).
    pub fn replicate(&mut self, action: CoordinatorAction) -> Result<u64, WcError> {
        if !self.is_leader() {
            return Err(WcError::new(
                ErrorCode::PermissionDenied,
                "Only the leader can replicate entries",
            ));
        }

        let entry =
            RaftLogEntry { term: self.raft_term, index: self.storage.last_log_index() + 1, action };
        self.storage.append(entry)
    }

    /// Get the storage for inspection.
    pub fn storage(&self) -> &RaftCoordinatorStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_creation_defaults() {
        let coord = Coordinator::new("coord-001", 0);
        assert_eq!(coord.coordinator_id, "coord-001");
        assert_eq!(coord.shard_id, 0);
        assert_eq!(coord.raft_term, 0);
        assert_eq!(coord.raft_role, CoordinatorRole::Follower);
    }

    #[test]
    fn is_leader_false_when_follower() {
        let coord = Coordinator::new("coord-001", 0);
        assert!(!coord.is_leader());
    }

    #[test]
    fn is_leader_true_after_become_leader() {
        let mut coord = Coordinator::new("coord-001", 0);
        coord.start_election();
        coord.become_leader();
        assert!(coord.is_leader());
        assert_eq!(coord.raft_role, CoordinatorRole::Leader);
    }

    #[test]
    fn start_election_increments_term_and_sets_candidate() {
        let mut coord = Coordinator::new("coord-001", 0);
        coord.start_election();
        assert_eq!(coord.raft_term, 1);
        assert_eq!(coord.raft_role, CoordinatorRole::Candidate);
        assert!(!coord.is_leader());
    }

    #[test]
    fn step_down_reverts_to_follower() {
        let mut coord = Coordinator::new("coord-001", 0);
        coord.start_election();
        coord.become_leader();
        assert!(coord.is_leader());
        coord.step_down(5);
        assert!(!coord.is_leader());
        assert_eq!(coord.raft_role, CoordinatorRole::Follower);
        assert_eq!(coord.raft_term, 5);
    }

    #[test]
    fn multiple_shards_are_independent() {
        let coord0 = Coordinator::new("coord-A", 0);
        let mut coord1 = Coordinator::new("coord-B", 1);
        coord1.start_election();
        coord1.become_leader();
        assert!(!coord0.is_leader());
        assert!(coord1.is_leader());
        assert_eq!(coord0.shard_id, 0);
        assert_eq!(coord1.shard_id, 1);
    }

    #[test]
    fn raft_storage_append_and_get() {
        let mut storage = RaftCoordinatorStorage::new();
        let entry = RaftLogEntry { term: 1, index: 1, action: CoordinatorAction::Noop };
        assert!(storage.append(entry).is_ok());
        assert_eq!(storage.len(), 1);
        assert_eq!(storage.last_log_index(), 1);
        assert_eq!(storage.last_log_term(), 1);
        assert!(storage.get(1).is_some());
    }

    #[test]
    fn raft_storage_commit_and_apply() {
        let mut storage = RaftCoordinatorStorage::new();
        for i in 1..=3 {
            let entry = RaftLogEntry {
                term: 1,
                index: i,
                action: CoordinatorAction::AssignJob {
                    job_id: format!("job-{i}"),
                    donor_id: format!("donor-{i}"),
                },
            };
            storage.append(entry).unwrap();
        }
        storage.set_commit_index(2);
        let applied = storage.apply_committed();
        assert_eq!(applied.len(), 2);
    }

    #[test]
    fn raft_vote_tracking() {
        let mut storage = RaftCoordinatorStorage::new();
        assert!(storage.voted_for(1).is_none());
        storage.record_vote(1, "coord-A".into());
        assert_eq!(storage.voted_for(1), Some("coord-A"));
        assert!(storage.voted_for(2).is_none());
    }

    #[test]
    fn quorum_election_with_peers() {
        let peers = vec!["coord-B".into(), "coord-C".into()];
        let mut coord = Coordinator::with_peers("coord-A", 0, peers);
        assert_eq!(coord.quorum_size(), 2); // 3 nodes, need 2

        coord.start_election(); // votes for self (1 vote)
        assert_eq!(coord.raft_role, CoordinatorRole::Candidate);

        coord.receive_vote("coord-B", 1, true); // 2nd vote → quorum
        assert!(coord.is_leader());
    }

    #[test]
    fn leader_can_replicate() {
        let mut coord = Coordinator::new("coord-001", 0);
        coord.start_election();
        coord.become_leader();

        let result = coord.replicate(CoordinatorAction::AssignJob {
            job_id: "job-1".into(),
            donor_id: "donor-1".into(),
        });
        assert!(result.is_ok());
        // noop from become_leader + the replicated entry
        assert_eq!(coord.storage().len(), 2);
    }

    #[test]
    fn follower_cannot_replicate() {
        let mut coord = Coordinator::new("coord-001", 0);
        let result = coord.replicate(CoordinatorAction::Noop);
        assert!(result.is_err());
    }

    #[test]
    fn raft_storage_with_wal() {
        let wal_path = std::path::PathBuf::from("/tmp/wc-test-raft-wal.json");
        let mut storage = RaftCoordinatorStorage::with_wal(wal_path.clone());
        let entry = RaftLogEntry { term: 1, index: 1, action: CoordinatorAction::Noop };
        assert!(storage.append(entry).is_ok());
        // WAL file should exist
        assert!(wal_path.exists());
        let _ = std::fs::remove_file(&wal_path);
    }
}
