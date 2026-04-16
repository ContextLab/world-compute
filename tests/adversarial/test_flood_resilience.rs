//! Adversarial test: flood resilience — malformed peer message flood.
//!
//! These tests require a live P2P network layer and must NOT run in normal CI.
//! Run manually: `cargo test --test test_flood_resilience -- --ignored`

/// Verify that a flood of malformed gossip messages does not crash the node.
///
/// This test will:
/// 1. Connect a test peer directly to the node under test.
/// 2. Send 100_000 randomly-malformed gossip protocol frames as fast as
///    possible over the connection.
/// 3. Assert the node remains responsive to legitimate heartbeat probes
///    throughout and after the flood.
/// 4. Assert no panic, no unbounded memory growth, and no legitimate
///    messages are dropped.
///
/// Requires: live libp2p transport layer, gossip subsystem, metrics endpoint.
#[test]
#[ignore]
fn malformed_peer_flood() {
    // TODO(T140): implement once gossip transport layer supports test injection.
    // Expected: node CPU usage stays below 80%; response latency to a probe
    // sent during the flood is < 500 ms; node logs show "malformed frame"
    // warnings but no panics.
    unimplemented!("Needs live gossip transport — run with --ignored lifted in integration env");
}

/// Verify that a job-submit flood is rate-limited and does not exhaust memory.
///
/// This test will:
/// 1. Submit 10_000 job manifests per second from a single caller.
/// 2. Assert the rate limiter kicks in after the 10th request per minute.
/// 3. Assert the node's memory usage stays bounded.
///
/// Requires: rate limiter active, scheduler accepting requests.
#[test]
#[ignore]
fn job_submit_flood_rate_limited() {
    unimplemented!("Needs live scheduler + rate limiter — run with --ignored lifted in integration env");
}
