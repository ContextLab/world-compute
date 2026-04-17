//! LAN testnet structural tests (T117-T120).
//!
//! Since we can't spawn real agent processes in a test, these tests verify
//! the structural types used for cluster formation, replication, checkpoint,
//! and preemption timing.

use std::time::Instant;
use worldcompute::data_plane::cid_store::{compute_cid, CidStore};
use worldcompute::preemption::supervisor::{PreemptionResult, PreemptionSupervisor};
use worldcompute::scheduler::broker::{Broker, NodeInfo, TaskRequirements};
use worldcompute::scheduler::ResourceEnvelope;
use worldcompute::types::Timestamp;

fn test_envelope(cpu: u64, ram: u64) -> ResourceEnvelope {
    ResourceEnvelope {
        cpu_millicores: cpu,
        ram_bytes: ram,
        gpu_class: None,
        gpu_vram_bytes: 0,
        scratch_bytes: 10 * 1024 * 1024 * 1024,
        network_egress_bytes: 0,
        walltime_budget_ms: 3_600_000,
    }
}

fn test_node(peer_id: &str) -> NodeInfo {
    NodeInfo {
        peer_id: peer_id.to_string(),
        region_code: "lan-local".to_string(),
        capacity: test_envelope(4000, 8 * 1024 * 1024 * 1024),
        trust_tier: 1,
        attestation_verified: false,
        attestation_verified_at: None,
    }
}

/// T117: Cluster formation — node roster management.
#[test]
fn cluster_formation_types() {
    let mut broker = Broker::new("lan-broker", "lan-local");

    // Register 5 nodes to form a LAN cluster
    for i in 0..5 {
        broker.register_node(test_node(&format!("node-{i}"))).unwrap();
    }
    assert_eq!(broker.node_roster.len(), 5);

    // All nodes should be matchable
    let reqs = TaskRequirements {
        min_cpu_millicores: 1000,
        min_ram_bytes: 1,
        min_scratch_bytes: 1,
        min_trust_tier: 1,
    };
    let matched = broker.match_task(&reqs).unwrap();
    assert_eq!(matched.len(), 5);
}

/// T118: R=3 replica placement — 3 different nodes selected.
#[test]
fn r3_replica_placement() {
    let mut broker = Broker::new("lan-broker", "lan-local");

    // Register 5 nodes
    for i in 0..5 {
        broker.register_node(test_node(&format!("replica-node-{i}"))).unwrap();
    }

    let reqs = TaskRequirements {
        min_cpu_millicores: 1000,
        min_ram_bytes: 1,
        min_scratch_bytes: 1,
        min_trust_tier: 1,
    };
    let matched = broker.match_task(&reqs).unwrap();

    // Select R=3 replicas from matched nodes
    let r = 3usize;
    assert!(
        matched.len() >= r,
        "Need at least {r} nodes for R={r} replication, got {}",
        matched.len()
    );

    // Verify selected replicas are distinct
    let replicas: Vec<&String> = matched.iter().take(r).collect();
    for i in 0..replicas.len() {
        for j in (i + 1)..replicas.len() {
            assert_ne!(replicas[i], replicas[j], "Replicas must be placed on different nodes");
        }
    }
}

/// T119: Checkpoint/resume flow — checkpoint struct creation, resume from CID.
#[test]
fn checkpoint_resume_flow_types() {
    let store = CidStore::new();

    // Simulate checkpoint: serialize state and store
    let checkpoint_data = b"serialized task state at step 42";
    let checkpoint_cid = store.put(checkpoint_data).unwrap();
    assert!(store.has(&checkpoint_cid));

    // Simulate resume: retrieve checkpoint by CID
    let restored = store.get(&checkpoint_cid).unwrap();
    assert_eq!(restored, checkpoint_data);

    // Verify CID is deterministic (same state produces same checkpoint CID)
    let cid2 = compute_cid(checkpoint_data).unwrap();
    assert_eq!(checkpoint_cid, cid2);
}

/// T120: Preemption timing — verify Instant-based timing works within budget.
#[test]
fn preemption_timing_assertions() {
    let start = Instant::now();

    // Simulate a freeze operation (no real sandboxes)
    let (_tx, rx) = tokio::sync::watch::channel(None);
    let mut supervisor = PreemptionSupervisor::new(rx);
    let result = supervisor.freeze_all();

    let elapsed_us = start.elapsed().as_micros() as u64;

    // With no sandboxes, freeze should be near-instant (well under 10ms budget)
    assert!(result.within_budget(), "Empty freeze should be within 10ms budget");
    assert!(
        elapsed_us < 1_000_000, // 1 second max for the whole test
        "Preemption timing test took too long: {elapsed_us}us"
    );
}
