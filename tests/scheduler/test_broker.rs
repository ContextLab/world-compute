//! Integration tests for broker types (T109).

use worldcompute::scheduler::broker::{Broker, Lease, LeaseStatus, NodeInfo, TaskRequirements};
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

fn test_node(peer_id: &str, cpu: u64, ram: u64) -> NodeInfo {
    NodeInfo {
        peer_id: peer_id.to_string(),
        region_code: "us-east-1".to_string(),
        capacity: test_envelope(cpu, ram),
        trust_tier: 1,
        attestation_verified: false,
        attestation_verified_at: None,
    }
}

#[test]
fn lease_creation() {
    let lease = Lease {
        lease_id: "lease-001".into(),
        task_id: "task-001".into(),
        node_id: libp2p::PeerId::random(),
        issued_at: Timestamp::now(),
        ttl_ms: 30_000,
        renewed_at: None,
        status: LeaseStatus::Active,
    };
    assert_eq!(lease.lease_id, "lease-001");
    assert_eq!(lease.ttl_ms, 30_000);
    assert!(matches!(lease.status, LeaseStatus::Active));
}

#[test]
fn lease_status_transitions() {
    let mut lease = Lease {
        lease_id: "lease-002".into(),
        task_id: "task-002".into(),
        node_id: libp2p::PeerId::random(),
        issued_at: Timestamp::now(),
        ttl_ms: 60_000,
        renewed_at: None,
        status: LeaseStatus::Active,
    };
    assert!(matches!(lease.status, LeaseStatus::Active));

    lease.status = LeaseStatus::Expired;
    assert!(matches!(lease.status, LeaseStatus::Expired));

    lease.status = LeaseStatus::Released;
    assert!(matches!(lease.status, LeaseStatus::Released));
}

#[test]
fn lease_ttl_values() {
    let short_ttl = 5_000u64;
    let long_ttl = 300_000u64;
    assert!(long_ttl > short_ttl);

    let lease = Lease {
        lease_id: "lease-003".into(),
        task_id: "task-003".into(),
        node_id: libp2p::PeerId::random(),
        issued_at: Timestamp::now(),
        ttl_ms: short_ttl,
        renewed_at: None,
        status: LeaseStatus::Active,
    };
    assert_eq!(lease.ttl_ms, short_ttl);
}

#[test]
fn broker_register_and_match() {
    let mut broker = Broker::new("broker-integ", "us-west-2");
    broker.register_node(test_node("peer-big", 8000, 16 * 1024 * 1024 * 1024)).unwrap();
    broker.register_node(test_node("peer-small", 1000, 1024 * 1024 * 1024)).unwrap();

    let reqs = TaskRequirements {
        min_cpu_millicores: 4000,
        min_ram_bytes: 8 * 1024 * 1024 * 1024,
        min_scratch_bytes: 1,
        min_trust_tier: 1,
    };
    let matched = broker.match_task(&reqs).unwrap();
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0], "peer-big");
}
