//! T025 [US1]: Scratch space fully reclaimed after job termination.

use worldcompute::sandbox::Sandbox;

#[test]
fn scratch_space_reclaimed_after_terminate_and_cleanup() {
    use worldcompute::sandbox::firecracker::FirecrackerSandbox;
    let tmp = std::env::temp_dir().join("wc-t025-scratch");
    let _ = std::fs::remove_dir_all(&tmp); // clean up from previous runs
    std::fs::create_dir_all(tmp.join("scratch")).unwrap();
    // Simulate 10MB scratch data
    let data = vec![0xABu8; 10 * 1024 * 1024];
    std::fs::write(tmp.join("scratch/output.bin"), &data).unwrap();
    assert!(tmp.join("scratch/output.bin").exists());

    let mut sandbox = FirecrackerSandbox::new(tmp.clone());
    sandbox.terminate().unwrap();
    sandbox.cleanup().unwrap();

    assert!(!tmp.exists(), "Scratch space must be fully reclaimed");
}

#[test]
fn cleanup_on_empty_dir_succeeds() {
    use worldcompute::sandbox::firecracker::FirecrackerSandbox;
    let tmp = std::env::temp_dir().join("wc-t025-empty");
    let _ = std::fs::remove_dir_all(&tmp);
    let mut sandbox = FirecrackerSandbox::new(tmp);
    assert!(sandbox.cleanup().is_ok());
}
