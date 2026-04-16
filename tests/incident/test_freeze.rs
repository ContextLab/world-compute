//! T072 [US5]: FreezeHost removes host from scheduling pool.

use worldcompute::scheduler::broker::{Broker, NodeInfo, TaskRequirements};
use worldcompute::scheduler::ResourceEnvelope;

fn test_node(peer_id: &str) -> NodeInfo {
    NodeInfo {
        peer_id: peer_id.into(), region_code: "us-east-1".into(),
        capacity: ResourceEnvelope { cpu_millicores: 8000, ram_bytes: 16*1024*1024*1024, gpu_class: None, gpu_vram_bytes: 0, scratch_bytes: 10*1024*1024*1024, network_egress_bytes: 0, walltime_budget_ms: 3_600_000 },
        trust_tier: 1, attestation_verified: false, attestation_verified_at: None,
    }
}

#[test]
fn frozen_host_excluded_from_matching() {
    let mut broker = Broker::new("b1", "us-east-1");
    broker.register_node(test_node("peer-frozen")).unwrap();
    broker.register_node(test_node("peer-active")).unwrap();
    broker.freeze_host(&"peer-frozen".into());

    let reqs = TaskRequirements { min_cpu_millicores: 1000, min_ram_bytes: 1, min_scratch_bytes: 1, min_trust_tier: 1 };
    let matched = broker.match_task(&reqs).unwrap();
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0], "peer-active");
}

#[test]
fn unfreeze_restores_host() {
    let mut broker = Broker::new("b1", "us-east-1");
    broker.register_node(test_node("peer-1")).unwrap();
    broker.freeze_host(&"peer-1".into());
    broker.unfreeze_host(&"peer-1".into());

    let reqs = TaskRequirements { min_cpu_millicores: 1000, min_ram_bytes: 1, min_scratch_bytes: 1, min_trust_tier: 1 };
    assert_eq!(broker.match_task(&reqs).unwrap().len(), 1);
}
