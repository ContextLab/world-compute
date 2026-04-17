//! T024 [US1]: Host filesystem invisible from guest.
//!
//! Verifies sandbox cleanup leaves no host residue (FR-S003).

use worldcompute::sandbox::Sandbox;

#[test]
fn firecracker_cleanup_removes_all_files() {
    use worldcompute::sandbox::firecracker::FirecrackerSandbox;
    let tmp = std::env::temp_dir().join(format!("wc-t024-fc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("secret.txt"), b"host data").unwrap();

    let mut sandbox = FirecrackerSandbox::new(tmp.clone());
    sandbox.cleanup().unwrap();
    assert!(!tmp.exists(), "No host residue after cleanup");
}

#[test]
fn apple_vf_cleanup_removes_all_files() {
    use worldcompute::sandbox::apple_vf::AppleVfSandbox;
    let tmp = std::env::temp_dir().join("wc-t024-vf");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("secret.txt"), b"host data").unwrap();

    let mut sandbox = AppleVfSandbox::new(tmp.clone());
    sandbox.cleanup().unwrap();
    assert!(!tmp.exists(), "No host residue after cleanup");
}

#[test]
fn hyperv_cleanup_removes_all_files() {
    use worldcompute::sandbox::hyperv::HyperVSandbox;
    let tmp = std::env::temp_dir().join("wc-t024-hv");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("secret.txt"), b"host data").unwrap();

    let mut sandbox = HyperVSandbox::new(tmp.clone());
    sandbox.cleanup().unwrap();
    assert!(!tmp.exists(), "No host residue after cleanup");
}
