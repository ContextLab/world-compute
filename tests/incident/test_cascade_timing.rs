//! T081 [US5]: Simulate sandbox anomaly, verify full containment cascade
//! completes within 60 seconds (SC-S006).

use std::time::Instant;
use worldcompute::incident::containment::execute_containment;
use worldcompute::incident::ContainmentAction;
use worldcompute::scheduler::broker::{Broker, NodeInfo};
use worldcompute::scheduler::ResourceEnvelope;

fn test_node(peer_id: &str) -> NodeInfo {
    NodeInfo {
        peer_id: peer_id.into(),
        region_code: "us-east-1".into(),
        capacity: ResourceEnvelope {
            cpu_millicores: 8000, ram_bytes: 16 * 1024 * 1024 * 1024,
            gpu_class: None, gpu_vram_bytes: 0,
            scratch_bytes: 10 * 1024 * 1024 * 1024,
            network_egress_bytes: 0, walltime_budget_ms: 3_600_000,
        },
        trust_tier: 1,
        attestation_verified: true,
        attestation_verified_at: Some(0),
    }
}

/// Simulate a full containment cascade:
/// 1. Detect anomaly (simulated)
/// 2. FreezeHost — remove from scheduling
/// 3. QuarantineWorkloadClass — block all jobs of this class
/// 4. Log IncidentRecord for each action
/// 5. Verify frozen host is excluded from matching
///
/// All steps must complete within 60 seconds (SC-S006).
#[test]
fn containment_cascade_completes_within_60_seconds() {
    let start = Instant::now();

    // Setup: broker with nodes
    let mut broker = Broker::new("broker-001", "us-east-1");
    broker.register_node(test_node("peer-compromised")).unwrap();
    broker.register_node(test_node("peer-healthy")).unwrap();

    // Step 1: Anomaly detected on peer-compromised
    let anomaly_detected = Instant::now();

    // Step 2: FreezeHost
    let freeze_record = execute_containment(
        ContainmentAction::FreezeHost,
        "peer-compromised",
        "peer-oncall",
        "OnCallResponder",
        "Repeated denied syscalls detected — possible sandbox escape attempt",
        "incident-cascade-001",
    ).unwrap();
    assert!(freeze_record.reversible);
    broker.freeze_host(&"peer-compromised".into());

    // Step 3: QuarantineWorkloadClass
    let quarantine_record = execute_containment(
        ContainmentAction::QuarantineWorkloadClass,
        "Scientific",
        "peer-oncall",
        "OnCallResponder",
        "Workload class associated with anomaly on peer-compromised",
        "incident-cascade-001",
    ).unwrap();
    assert!(quarantine_record.reversible);

    // Step 4: Verify frozen host excluded from scheduling
    let reqs = worldcompute::scheduler::broker::TaskRequirements {
        min_cpu_millicores: 1000,
        min_ram_bytes: 1,
        min_scratch_bytes: 1,
        min_trust_tier: 1,
    };
    let matched = broker.match_task(&reqs).unwrap();
    assert_eq!(matched.len(), 1, "Only healthy peer should be matchable");
    assert_eq!(matched[0], "peer-healthy");

    // Step 5: Verify cascade completed within 60 seconds
    let cascade_duration = start.elapsed();
    let anomaly_to_containment = anomaly_detected.elapsed();

    assert!(
        cascade_duration.as_secs() < 60,
        "Full cascade took {:?} — must complete within 60 seconds (SC-S006)",
        cascade_duration
    );

    // In practice this completes in microseconds since it's all in-memory.
    // The 60-second budget is for real deployments with network calls.
    assert!(
        anomaly_to_containment.as_millis() < 1000,
        "Anomaly-to-containment took {:?} — should be sub-second for in-memory ops",
        anomaly_to_containment
    );

    // Step 6: Verify audit trail completeness
    assert_eq!(freeze_record.incident_id, "incident-cascade-001");
    assert_eq!(quarantine_record.incident_id, "incident-cascade-001");
    assert_eq!(freeze_record.actor_role, "OnCallResponder");
    assert!(!freeze_record.justification.is_empty());
    assert!(!quarantine_record.justification.is_empty());
}

#[test]
fn containment_reversal_works() {
    let mut broker = Broker::new("broker-001", "us-east-1");
    broker.register_node(test_node("peer-1")).unwrap();
    broker.freeze_host(&"peer-1".into());

    // Verify frozen
    assert!(broker.is_host_frozen(&"peer-1".into()));

    // Execute LiftFreeze
    let lift_record = execute_containment(
        ContainmentAction::LiftFreeze,
        "peer-1",
        "peer-oncall",
        "OnCallResponder",
        "Investigation complete — no compromise found",
        "incident-cascade-001",
    ).unwrap();
    broker.unfreeze_host(&"peer-1".into());

    // Verify unfrozen
    assert!(!broker.is_host_frozen(&"peer-1".into()));
    assert!(lift_record.reversible);
}
