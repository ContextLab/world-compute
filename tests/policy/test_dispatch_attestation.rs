//! T038 [US2]: Forged TPM2 quote rejected at dispatch time.

use worldcompute::scheduler::broker::{Broker, NodeInfo};
use worldcompute::scheduler::ResourceEnvelope;
use worldcompute::types::{AttestationQuote, AttestationType};
use worldcompute::verification::attestation::MeasurementRegistry;

fn test_envelope() -> ResourceEnvelope {
    ResourceEnvelope {
        cpu_millicores: 4000,
        ram_bytes: 8 * 1024 * 1024 * 1024,
        gpu_class: None,
        gpu_vram_bytes: 0,
        scratch_bytes: 10 * 1024 * 1024 * 1024,
        network_egress_bytes: 0,
        walltime_budget_ms: 3_600_000,
    }
}

fn test_node() -> NodeInfo {
    NodeInfo {
        peer_id: "peer-test".into(),
        region_code: "us-east-1".into(),
        capacity: test_envelope(),
        trust_tier: 2,
        attestation_verified: false,
        attestation_verified_at: None,
    }
}

#[test]
fn forged_tpm2_quote_rejected_at_registration() {
    let mut broker = Broker::new("broker-001", "us-east-1");
    let registry = MeasurementRegistry::new();
    let node = test_node();

    // Non-empty but garbage TPM2 quote
    let forged_quote = AttestationQuote {
        quote_type: AttestationType::Tpm2,
        quote_bytes: vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00],
        platform_info: "test".into(),
    };

    let result = broker.register_node_with_attestation(node, &forged_quote, &registry);
    assert!(result.is_err(), "Forged TPM2 quote must be rejected at dispatch");
}

#[test]
fn empty_quote_downgrades_to_t0() {
    let mut broker = Broker::new("broker-001", "us-east-1");
    let registry = MeasurementRegistry::new();
    let mut node = test_node();
    node.trust_tier = 3; // claims T3

    let empty_quote = AttestationQuote {
        quote_type: AttestationType::Tpm2,
        quote_bytes: Vec::new(),
        platform_info: "test".into(),
    };

    broker.register_node_with_attestation(node, &empty_quote, &registry).unwrap();
    assert_eq!(broker.node_roster[0].trust_tier, 0, "Empty quote must downgrade to T0");
}
