//! Integration tests for scheduler matchmaking and leases (T132-T137).

use worldcompute::scheduler::broker::{
    check_lease_expiry, issue_lease, match_task, renew_lease, select_disjoint_replicas,
    NodeCapability, TaskRequirement,
};

fn gpu_node(id: &str, as_num: u32) -> NodeCapability {
    NodeCapability {
        node_id: id.to_string(),
        cpu_cores: 8,
        gpu_available: true,
        memory_mb: 16384,
        trust_tier: 2,
        autonomous_system: as_num,
    }
}

fn cpu_node(id: &str, as_num: u32) -> NodeCapability {
    NodeCapability {
        node_id: id.to_string(),
        cpu_cores: 4,
        gpu_available: false,
        memory_mb: 8192,
        trust_tier: 1,
        autonomous_system: as_num,
    }
}

#[test]
fn match_gpu_task_to_gpu_node() {
    let nodes = vec![cpu_node("cpu-1", 100), gpu_node("gpu-1", 200), cpu_node("cpu-2", 300)];
    let task = TaskRequirement {
        min_cpu_cores: 4,
        needs_gpu: true,
        min_memory_mb: 8192,
        min_trust_tier: 1,
    };
    let matched = match_task(&task, &nodes);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].node_id, "gpu-1");
}

#[test]
fn match_cpu_task_returns_all_eligible() {
    let nodes = vec![cpu_node("cpu-1", 100), gpu_node("gpu-1", 200), cpu_node("cpu-2", 300)];
    let task = TaskRequirement {
        min_cpu_cores: 2,
        needs_gpu: false,
        min_memory_mb: 4096,
        min_trust_tier: 1,
    };
    let matched = match_task(&task, &nodes);
    assert_eq!(matched.len(), 3);
}

#[test]
fn match_trust_tier_filter() {
    let nodes = vec![cpu_node("low-trust", 100), gpu_node("high-trust", 200)];
    let task = TaskRequirement {
        min_cpu_cores: 1,
        needs_gpu: false,
        min_memory_mb: 1024,
        min_trust_tier: 2,
    };
    let matched = match_task(&task, &nodes);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].node_id, "high-trust");
}

#[test]
fn lease_lifecycle() {
    let peer_id = libp2p::PeerId::random();

    // Issue lease with 100ms TTL
    let mut lease = issue_lease("task-1", peer_id, 100);
    assert_eq!(lease.task_id, "task-1");
    assert_eq!(lease.node_id, peer_id);
    assert!(lease.renewed_at.is_none());

    // Immediately after issue, lease should not be expired
    // (TTL is 100ms = 100_000 microseconds)
    assert!(!check_lease_expiry(&lease), "Lease should not be expired immediately");

    // Renew
    renew_lease(&mut lease);
    assert!(lease.renewed_at.is_some());
    assert!(!check_lease_expiry(&lease), "Lease should not be expired after renewal");
}

#[test]
fn expired_lease_detected() {
    let peer_id = libp2p::PeerId::random();
    // Issue a lease with 0ms TTL — it should be expired immediately
    let lease = issue_lease("task-expire", peer_id, 0);
    // Give a tiny margin — 0ms TTL means it expires at issue time
    assert!(check_lease_expiry(&lease), "Zero-TTL lease should be expired");
}

#[test]
fn disjoint_as_selection() {
    let nodes = vec![
        gpu_node("n1", 100),
        gpu_node("n2", 100), // same AS as n1
        gpu_node("n3", 200),
        gpu_node("n4", 300),
        gpu_node("n5", 300), // same AS as n4
    ];
    let refs: Vec<&NodeCapability> = nodes.iter().collect();
    let selected = select_disjoint_replicas(&refs, 3);
    assert_eq!(selected.len(), 3);

    // All selected should have different AS numbers
    let as_numbers: Vec<u32> = selected.iter().map(|n| n.autonomous_system).collect();
    let unique: std::collections::HashSet<u32> = as_numbers.iter().copied().collect();
    assert_eq!(unique.len(), 3, "All replicas must be from different AS: {as_numbers:?}");
}

#[test]
fn disjoint_selection_fewer_than_requested() {
    let nodes = [gpu_node("n1", 100), gpu_node("n2", 100)];
    let refs: Vec<&NodeCapability> = nodes.iter().collect();
    // Request 3 but only 1 distinct AS available
    let selected = select_disjoint_replicas(&refs, 3);
    assert_eq!(selected.len(), 1);
}
