//! Adversarial test: sandbox escape via filesystem access.
//!
//! These tests require a live sandbox runtime and must NOT run in normal CI.
//! Run manually: `cargo test --test test_sandbox_escape -- --ignored`

/// Verify that a WASM workload cannot read /etc/passwd from the host.
///
/// This test will:
/// 1. Spawn a sandboxed WASM job that attempts to open "/etc/passwd".
/// 2. Assert the job receives a permission-denied error (not the file contents).
/// 3. Confirm the sandbox audit log records the denied syscall.
///
/// Requires: wasmtime sandbox runtime, seccomp-bpf filter active.
#[test]
#[ignore]
fn sandbox_read_etc_passwd() {
    // TODO(T137): implement once sandbox runtime integration is available.
    // Expected: the job execution returns SandboxUnavailable or the job
    // output contains no host-filesystem data. The seccomp log should show
    // a blocked openat(2) call for the host path.
    unimplemented!("Needs live sandbox runtime — run with --ignored lifted in integration env");
}

/// Verify that a container workload cannot pivot_root or chroot to escape.
///
/// This test will:
/// 1. Submit an OCI job that calls pivot_root(2) inside the container.
/// 2. Assert the syscall is blocked by the seccomp profile.
///
/// Requires: OCI runtime with seccomp profile enforced.
#[test]
#[ignore]
fn sandbox_pivot_root_blocked() {
    unimplemented!("Needs OCI runtime with seccomp profile — run with --ignored lifted in integration env");
}
