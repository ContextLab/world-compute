//! T041: Integration test for Ed25519 policy verification with real key pairs.
//!
//! Creates a manifest, signs it with ed25519_dalek, and verifies it passes
//! check_signature(). Also tests that a wrong-key signature fails.

use ed25519_dalek::{Signer, SigningKey};
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::policy::rules::{check_signature, manifest_signing_bytes};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn make_manifest() -> JobManifest {
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"test artifact").unwrap();
    JobManifest {
        manifest_cid: None,
        name: "ed25519-test-job".into(),
        workload_type: WorkloadType::WasmModule,
        workload_cid: cid,
        command: vec!["run".into()],
        inputs: Vec::new(),
        output_sink: "cid-store".into(),
        resources: ResourceEnvelope {
            cpu_millicores: 1000,
            ram_bytes: 512 * 1024 * 1024,
            gpu_class: None,
            gpu_vram_bytes: 0,
            scratch_bytes: 1024 * 1024 * 1024,
            network_egress_bytes: 0,
            walltime_budget_ms: 3_600_000,
        },
        category: JobCategory::PublicGood,
        confidentiality: ConfidentialityLevel::Public,
        verification: VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000,
        submitter_signature: vec![0u8; 64], // placeholder, will be replaced
    }
}

fn make_ctx(verifying_key_bytes: &[u8]) -> SubmissionContext {
    SubmissionContext {
        submitter_peer_id: "12D3KooWTestEd25519".into(),
        submitter_public_key: verifying_key_bytes.to_vec(),
        submitter_hp_score: 10,
        submitter_banned: false,
        epoch_submission_count: 0,
        epoch_submission_quota: 100,
    }
}

// ─── Correct key: signature verification passes ────────────────────────

#[test]
fn correct_ed25519_signature_passes_check_signature() {
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut manifest = make_manifest();
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();

    let ctx = make_ctx(&verifying_key.to_bytes());
    let check = check_signature(&manifest, &ctx);
    assert!(check.passed, "check_signature must pass with correct Ed25519 key: {}", check.detail);
}

// ─── Wrong key: signature verification fails ───────────────────────────

#[test]
fn wrong_ed25519_key_fails_check_signature() {
    // Sign with key A
    let signing_key_a = SigningKey::from_bytes(&[42u8; 32]);

    let mut manifest = make_manifest();
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key_a.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();

    // Verify with key B (different key)
    let signing_key_b = SigningKey::from_bytes(&[99u8; 32]);
    let verifying_key_b = signing_key_b.verifying_key();

    let ctx = make_ctx(&verifying_key_b.to_bytes());
    let check = check_signature(&manifest, &ctx);
    assert!(!check.passed, "check_signature must FAIL when signature is from a different key");
}

// ─── Correct key through full policy engine ─────────────────────────────

#[test]
fn correct_signature_passes_full_policy_engine() {
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut manifest = make_manifest();
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();

    let ctx = make_ctx(&verifying_key.to_bytes());
    let decision = evaluate(&manifest, &ctx).unwrap();
    assert_eq!(
        decision.verdict,
        Verdict::Accept,
        "Full policy engine should accept correctly-signed manifest"
    );
}

// ─── Wrong key through full policy engine ───────────────────────────────

#[test]
fn wrong_key_rejected_by_full_policy_engine() {
    let signing_key_a = SigningKey::from_bytes(&[42u8; 32]);

    let mut manifest = make_manifest();
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key_a.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();

    // Use a different key for verification context
    let signing_key_b = SigningKey::from_bytes(&[99u8; 32]);
    let verifying_key_b = signing_key_b.verifying_key();

    let ctx = make_ctx(&verifying_key_b.to_bytes());
    let decision = evaluate(&manifest, &ctx).unwrap();
    assert_eq!(
        decision.verdict,
        Verdict::Reject,
        "Full policy engine should reject manifest signed with wrong key"
    );
}

// ─── Empty signature rejected ───────────────────────────────────────────

#[test]
fn empty_signature_rejected() {
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut manifest = make_manifest();
    manifest.submitter_signature = Vec::new();

    let ctx = make_ctx(&verifying_key.to_bytes());
    let check = check_signature(&manifest, &ctx);
    assert!(!check.passed, "Empty signature must be rejected");
}

// ─── All-zero signature rejected ────────────────────────────────────────

#[test]
fn all_zero_signature_rejected() {
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut manifest = make_manifest();
    manifest.submitter_signature = vec![0u8; 64];

    let ctx = make_ctx(&verifying_key.to_bytes());
    let check = check_signature(&manifest, &ctx);
    assert!(!check.passed, "All-zero signature must be rejected");
}

// ─── Tampered manifest rejected ─────────────────────────────────────────

#[test]
fn tampered_manifest_rejected() {
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut manifest = make_manifest();
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();

    // Tamper with the manifest after signing
    manifest.name = "tampered-job".into();

    let ctx = make_ctx(&verifying_key.to_bytes());
    let check = check_signature(&manifest, &ctx);
    assert!(!check.passed, "Tampered manifest must fail signature verification");
}
