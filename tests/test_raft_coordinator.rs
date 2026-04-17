//! T065: Integration test for Raft coordinator — quorum election, log replication, step_down.
//!
//! Tests the coordinator through its public API as an integration consumer,
//! exercising the full election-replicate-stepdown lifecycle.

use worldcompute::scheduler::coordinator::{Coordinator, CoordinatorAction, CoordinatorRole};

#[test]
fn quorum_election_with_3_peers_and_replication() {
    // Set up a 3-node cluster: coord-A, coord-B, coord-C
    let peers = vec!["coord-B".into(), "coord-C".into()];
    let mut leader = Coordinator::with_peers("coord-A", 0, peers);

    // Initially a follower
    assert_eq!(leader.raft_role, CoordinatorRole::Follower);
    assert!(!leader.is_leader());
    assert_eq!(leader.quorum_size(), 2); // majority of 3

    // Start election — votes for self (1 vote)
    leader.start_election();
    assert_eq!(leader.raft_role, CoordinatorRole::Candidate);
    assert_eq!(leader.raft_term, 1);

    // Denied vote doesn't change state
    leader.receive_vote("coord-B", 1, false);
    assert_eq!(leader.raft_role, CoordinatorRole::Candidate);

    // Granted vote from coord-C reaches quorum (2 of 3)
    leader.receive_vote("coord-C", 1, true);
    assert!(leader.is_leader());
    assert_eq!(leader.raft_role, CoordinatorRole::Leader);

    // Leader can replicate entries
    let idx = leader
        .replicate(CoordinatorAction::AssignJob {
            job_id: "job-100".into(),
            donor_id: "donor-X".into(),
        })
        .expect("leader should replicate");
    assert!(idx > 0);

    // Replicate more entries
    leader
        .replicate(CoordinatorAction::RegisterDonor { donor_id: "donor-Y".into(), shard_id: 0 })
        .expect("second replication should succeed");

    // Storage should contain noop + 2 replicated entries
    assert_eq!(leader.storage().len(), 3);
}

#[test]
fn step_down_on_higher_term() {
    let peers = vec!["coord-B".into(), "coord-C".into()];
    let mut coord = Coordinator::with_peers("coord-A", 0, peers);

    // Become leader in term 1
    coord.start_election();
    coord.receive_vote("coord-B", 1, true);
    assert!(coord.is_leader());

    // Step down when a higher term is observed
    coord.step_down(5);
    assert_eq!(coord.raft_role, CoordinatorRole::Follower);
    assert_eq!(coord.raft_term, 5);
    assert!(!coord.is_leader());

    // Follower cannot replicate
    let err = coord.replicate(CoordinatorAction::Noop);
    assert!(err.is_err());
}

#[test]
fn vote_from_wrong_term_is_ignored() {
    let peers = vec!["coord-B".into(), "coord-C".into()];
    let mut coord = Coordinator::with_peers("coord-A", 0, peers);

    coord.start_election(); // term 1
                            // Vote from a stale term should be ignored
    coord.receive_vote("coord-B", 0, true);
    assert_eq!(coord.raft_role, CoordinatorRole::Candidate);

    // Vote from future term also ignored (not matching current)
    coord.receive_vote("coord-C", 2, true);
    assert_eq!(coord.raft_role, CoordinatorRole::Candidate);
}
