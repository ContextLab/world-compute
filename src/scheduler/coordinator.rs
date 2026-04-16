//! Coordinator scaffold — Raft role management and shard coordination (T083-T084).
//!
//! The coordinator drives consensus for a scheduler shard.
//! Full Raft integration (log replication, elections) is stubbed here —
//! the types and role transitions are wired; the consensus engine plugs in later.

use serde::{Deserialize, Serialize};

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
}

impl Coordinator {
    /// Create a new coordinator starting as a Follower in term 0.
    pub fn new(coordinator_id: impl Into<String>, shard_id: u32) -> Self {
        Self {
            coordinator_id: coordinator_id.into(),
            shard_id,
            raft_term: 0,
            raft_role: CoordinatorRole::Follower,
        }
    }

    /// Returns true if this coordinator is currently the shard leader.
    pub fn is_leader(&self) -> bool {
        self.raft_role == CoordinatorRole::Leader
    }

    /// Transition to Candidate role and increment the term.
    ///
    /// Called when election timeout fires and this node starts campaigning.
    /// Stub: real implementation broadcasts RequestVote RPCs.
    pub fn start_election(&mut self) {
        self.raft_term += 1;
        self.raft_role = CoordinatorRole::Candidate;
    }

    /// Transition to Leader role.
    ///
    /// Called once quorum of votes received.
    /// Stub: real implementation sends initial AppendEntries (heartbeats).
    pub fn become_leader(&mut self) {
        self.raft_role = CoordinatorRole::Leader;
    }

    /// Step down to Follower, updating term if a higher term is seen.
    ///
    /// Called when a higher term is observed in any RPC.
    pub fn step_down(&mut self, new_term: u64) {
        if new_term >= self.raft_term {
            self.raft_term = new_term;
        }
        self.raft_role = CoordinatorRole::Follower;
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
}
