//! Red Team Scenario 3: Policy bypass attempt.
//!
//! Attack: Try to circumvent the deterministic policy engine — submit
//! without going through the pipeline, forge signatures, use expired
//! attestation, violate separation of duties.

use worldcompute::error::ErrorCode;
use worldcompute::governance::roles::{check_separation_of_duties, GovernanceRole, RoleType};
use worldcompute::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::scheduler::broker::{Broker, NodeInfo};
use worldcompute::scheduler::ResourceEnvelope;
use worldcompute::types::{AttestationQuote, AttestationType, Timestamp};
use worldcompute::verification::attestation::MeasurementRegistry;

fn bypass_manifest(sig: Vec<u8>) -> worldcompute::scheduler::manifest::JobManifest {
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"bypass-attempt").unwrap();
    worldcompute::scheduler::manifest::JobManifest {
        manifest_cid: None, name: "bypass".into(),
        workload_type: worldcompute::scheduler::WorkloadType::WasmModule,
        workload_cid: cid, command: vec!["run".into()],
        inputs: Vec::new(), output_sink: "cid-store".into(),
        resources: ResourceEnvelope {
            cpu_millicores: 1000, ram_bytes: 512*1024*1024, gpu_class: None,
            gpu_vram_bytes: 0, scratch_bytes: 1024*1024*1024,
            network_egress_bytes: 0, walltime_budget_ms: 3_600_000,
        },
        category: worldcompute::scheduler::JobCategory::PublicGood,
        confidentiality: worldcompute::scheduler::ConfidentialityLevel::Public,
        verification: worldcompute::scheduler::VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000, submitter_signature: sig,
    }
}

#[test]
fn attack_3a_forged_signature_rejected() {
    let d = evaluate(
        &bypass_manifest(vec![0u8; 64]),
        &SubmissionContext {
            submitter_peer_id: "12D3KooWForger".into(), submitter_public_key: vec![0; 32],
            submitter_hp_score: 10, submitter_banned: false,
            epoch_submission_count: 0, epoch_submission_quota: 100,
        },
    ).unwrap();
    assert_eq!(d.verdict, Verdict::Reject, "Forged (all-zero) signature must be rejected");
}

#[test]
fn attack_3b_empty_signature_rejected() {
    let d = evaluate(
        &bypass_manifest(Vec::new()),
        &SubmissionContext {
            submitter_peer_id: "12D3KooWForger".into(), submitter_public_key: vec![0; 32],
            submitter_hp_score: 10, submitter_banned: false,
            epoch_submission_count: 0, epoch_submission_quota: 100,
        },
    ).unwrap();
    assert_eq!(d.verdict, Verdict::Reject);
}

#[test]
fn attack_3c_forged_attestation_rejected_at_dispatch() {
    let mut broker = Broker::new("b1", "us-east-1");
    let registry = MeasurementRegistry::new();
    let node = NodeInfo {
        peer_id: "peer-forged".into(), region_code: "us-east-1".into(),
        capacity: ResourceEnvelope {
            cpu_millicores: 8000, ram_bytes: 16*1024*1024*1024, gpu_class: None,
            gpu_vram_bytes: 0, scratch_bytes: 10*1024*1024*1024,
            network_egress_bytes: 0, walltime_budget_ms: 3_600_000,
        },
        trust_tier: 3, attestation_verified: false, attestation_verified_at: None,
    };
    let forged_quote = AttestationQuote {
        quote_type: AttestationType::Tpm2,
        quote_bytes: vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00],
        platform_info: "forged".into(),
    };
    let result = broker.register_node_with_attestation(node, &forged_quote, &registry);
    assert!(result.is_err(), "Forged attestation must reject node registration");
}

#[test]
fn attack_3d_separation_of_duties_violation_blocked() {
    let existing = vec![
        GovernanceRole::new("r1".into(), "attacker".into(), RoleType::WorkloadApprover, "admin".into()),
    ];
    let err = check_separation_of_duties("attacker", RoleType::ArtifactSigner, &existing).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
}

#[test]
fn attack_3e_constitution_amendment_timelock_cannot_be_bypassed() {
    let mut proposal = GovernanceProposal {
        proposal_id: "p-bypass".into(), title: "Remove safety".into(),
        body: "Remove Principle I".into(), proposal_type: ProposalType::ConstitutionAmendment,
        state: ProposalState::Draft, submitter_id: "attacker".into(),
        created_at: Timestamp::now(), closes_at: Timestamp::now(),
        yes_votes: 100, no_votes: 0, abstain_votes: 0,
    };
    proposal.open_for_voting().unwrap();
    // Try to tally immediately — must fail due to 7-day review period
    let err = proposal.tally().unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
}
