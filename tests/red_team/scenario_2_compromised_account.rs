//! Red Team Scenario 2: Compromised account.
//!
//! Attack: Use a compromised/banned/low-HP account to submit jobs,
//! vote on safety-critical proposals, or perform admin actions.

use worldcompute::error::ErrorCode;
use worldcompute::governance::admin_service::AdminServiceHandler;
use worldcompute::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use worldcompute::governance::roles::{GovernanceRole, RoleType};
use worldcompute::governance::vote::{validate_vote_with_hp, Vote, VoteChoice};
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::types::Timestamp;

fn compromised_manifest() -> worldcompute::scheduler::manifest::JobManifest {
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"legit-looking").unwrap();
    worldcompute::scheduler::manifest::JobManifest {
        manifest_cid: None,
        name: "normal-job".into(),
        workload_type: worldcompute::scheduler::WorkloadType::WasmModule,
        workload_cid: cid,
        command: vec!["run".into()],
        inputs: Vec::new(),
        output_sink: "cid-store".into(),
        resources: worldcompute::scheduler::ResourceEnvelope {
            cpu_millicores: 1000,
            ram_bytes: 512 * 1024 * 1024,
            gpu_class: None,
            gpu_vram_bytes: 0,
            scratch_bytes: 1024 * 1024 * 1024,
            network_egress_bytes: 0,
            walltime_budget_ms: 3_600_000,
        },
        category: worldcompute::scheduler::JobCategory::PublicGood,
        confidentiality: worldcompute::scheduler::ConfidentialityLevel::Public,
        verification: worldcompute::scheduler::VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000,
        submitter_signature: vec![1u8; 64],
    }
}

#[test]
fn attack_2a_banned_account_cannot_submit() {
    let ctx = SubmissionContext {
        submitter_peer_id: "12D3KooWBanned".into(),
        submitter_public_key: vec![0; 32],
        submitter_hp_score: 10,
        submitter_banned: true,
        epoch_submission_count: 0,
        epoch_submission_quota: 100,
    };
    let d = evaluate(&compromised_manifest(), &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Reject);
    assert!(d.reject_reason.unwrap().contains("banned"));
}

#[test]
fn attack_2b_zero_hp_account_cannot_submit() {
    let ctx = SubmissionContext {
        submitter_peer_id: "12D3KooWSybil".into(),
        submitter_public_key: vec![0; 32],
        submitter_hp_score: 0,
        submitter_banned: false,
        epoch_submission_count: 0,
        epoch_submission_quota: 100,
    };
    let d = evaluate(&compromised_manifest(), &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Reject);
}

#[test]
fn attack_2c_low_hp_cannot_vote_on_emergency_halt() {
    let proposal = GovernanceProposal {
        proposal_id: "p-halt".into(),
        title: "Halt".into(),
        body: "Emergency".into(),
        proposal_type: ProposalType::EmergencyHalt,
        state: ProposalState::Open,
        submitter_id: "alice".into(),
        created_at: Timestamp::now(),
        closes_at: Timestamp::now(),
        yes_votes: 0,
        no_votes: 0,
        abstain_votes: 0,
    };
    let vote = Vote {
        vote_id: "v1".into(),
        proposal_id: "p-halt".into(),
        voter_id: "compromised".into(),
        choice: VoteChoice::Yes,
        weight: 1,
        signature: vec![],
        cast_at: Timestamp::now(),
    };
    let err = validate_vote_with_hp(&vote, &proposal, 3).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
}

#[test]
fn attack_2d_non_responder_cannot_halt_cluster() {
    let mut handler = AdminServiceHandler::new();
    let roles: Vec<GovernanceRole> = vec![];
    let err = handler.halt("takeover", "compromised-peer", &roles).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    assert!(!handler.halted);
}

#[test]
fn attack_2e_quota_flooding_blocked() {
    let ctx = SubmissionContext {
        submitter_peer_id: "12D3KooWFlooder".into(),
        submitter_public_key: vec![0; 32],
        submitter_hp_score: 10,
        submitter_banned: false,
        epoch_submission_count: 1000,
        epoch_submission_quota: 100,
    };
    let d = evaluate(&compromised_manifest(), &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Reject);
    assert!(d.reject_reason.unwrap().contains("quota"));
}
