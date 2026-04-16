//! Adversarial test: byzantine donor returning wrong results.
//!
//! These tests require a multi-node test cluster and must NOT run in normal CI.
//! Run manually: `cargo test --test test_byzantine_donor -- --ignored`

/// Verify that a donor returning a wrong computation result is detected.
///
/// This test will:
/// 1. Stand up a 5-node test cluster with one node configured as byzantine
///    (it XORs 0xFF into every output byte before returning).
/// 2. Submit a deterministic job (e.g., SHA-256 of a known input).
/// 3. Assert the verification layer detects the mismatch and marks the
///    byzantine node's trust score as penalised.
/// 4. Assert the correct result is still returned to the submitter via
///    quorum from the honest nodes.
///
/// Requires: multi-node test cluster, deterministic workload, verification
/// subsystem active.
#[test]
#[ignore]
fn wrong_result_injection() {
    // TODO(T139): implement once multi-node test harness is available.
    // Expected: WcError::QuorumFailure is NOT returned (honest quorum wins);
    // byzantine node's TrustScore drops below 0.3 after the round.
    unimplemented!("Needs multi-node test cluster — run with --ignored lifted in integration env");
}

/// Verify that a donor that selectively omits output shards is detected.
///
/// This test will:
/// 1. Configure one node to drop every third erasure-coded output shard.
/// 2. Assert the coordinator identifies the withholding node and retries
///    via another eligible node.
///
/// Requires: erasure coding active, coordinator liveness monitor.
#[test]
#[ignore]
fn shard_withholding_detected() {
    unimplemented!("Needs erasure-coding + coordinator monitor — run with --ignored lifted in integration env");
}
