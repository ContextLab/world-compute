//! Adversarial test: network isolation — workload cannot reach host network.
//!
//! These tests require a live sandbox runtime and must NOT run in normal CI.
//! Run manually: `cargo test --test test_network_isolation -- --ignored`

/// Verify that a sandboxed workload cannot probe the host network stack.
///
/// This test will:
/// 1. Launch a WASM/OCI job that attempts to open a raw socket and send
///    a probe packet to an RFC-5737 test address (192.0.2.1).
/// 2. Assert the socket(2) / connect(2) syscalls are blocked by the sandbox.
/// 3. Confirm no egress traffic appears on the host interface during the job.
///
/// Requires: network namespace isolation, seccomp socket filter, tcpdump
/// on the host loopback to detect leaks.
#[test]
#[ignore]
fn host_network_probe() {
    // TODO(T138): implement once network namespace plumbing is available.
    // Expected: socket(AF_INET, ...) returns EPERM; no packets observed
    // on host interface by external monitor.
    unimplemented!("Needs live sandbox with netns isolation — run with --ignored lifted in integration env");
}

/// Verify that DNS queries from within the sandbox are intercepted/blocked.
///
/// This test will:
/// 1. Submit a job that calls getaddrinfo("evil.example.com").
/// 2. Assert no DNS query reaches the host resolver.
///
/// Requires: sandbox DNS intercept policy enabled.
#[test]
#[ignore]
fn sandbox_dns_leak() {
    unimplemented!("Needs DNS intercept sandbox policy — run with --ignored lifted in integration env");
}
