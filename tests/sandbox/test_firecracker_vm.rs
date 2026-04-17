//! T030: Firecracker VM boot test — config validation.
//!
//! Tests that FirecrackerVmConfig validation works correctly:
//! valid configs pass, invalid configs fail. Does not actually
//! boot a VM since KVM may not be available in CI.

use std::path::PathBuf;
use worldcompute::sandbox::firecracker::{FirecrackerConfig, FirecrackerSandbox, FirecrackerVmConfig};

#[test]
fn vm_config_valid_values_accepted() {
    let cfg = FirecrackerVmConfig::new(
        2,
        512,
        PathBuf::from("/boot/vmlinux"),
        PathBuf::from("/tmp/rootfs.ext4"),
    );
    assert!(cfg.is_ok());
    let cfg = cfg.unwrap();
    assert_eq!(cfg.vcpu_count, 2);
    assert_eq!(cfg.mem_size_mib, 512);
}

#[test]
fn vm_config_minimum_values_accepted() {
    let cfg = FirecrackerVmConfig::new(
        1,
        128,
        PathBuf::from("/boot/vmlinux"),
        PathBuf::from("/tmp/rootfs.ext4"),
    );
    assert!(cfg.is_ok());
}

#[test]
fn vm_config_rejects_zero_vcpus() {
    let result = FirecrackerVmConfig::new(
        0,
        256,
        PathBuf::from("/boot/vmlinux"),
        PathBuf::from("/tmp/rootfs.ext4"),
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("vcpu_count"),
        "Error should mention vcpu_count"
    );
}

#[test]
fn vm_config_rejects_low_memory() {
    let result = FirecrackerVmConfig::new(
        1,
        64,
        PathBuf::from("/boot/vmlinux"),
        PathBuf::from("/tmp/rootfs.ext4"),
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("mem_size_mib"),
        "Error should mention mem_size_mib"
    );
}

#[test]
fn vm_config_boot_args_default() {
    let cfg = FirecrackerVmConfig::new(
        1,
        128,
        PathBuf::from("/boot/vmlinux"),
        PathBuf::from("/tmp/rootfs.ext4"),
    )
    .unwrap();
    assert_eq!(cfg.boot_args, "console=ttyS0 reboot=k panic=1 pci=off");
    assert_eq!(cfg.host_dev_name, "tap0");
}

#[test]
fn default_config_has_deny_all_egress() {
    let config = FirecrackerConfig::default();
    assert!(!config.egress_policy.egress_allowed, "Default egress must be deny-all");
    assert_eq!(config.vcpu_count, 1);
    assert_eq!(config.mem_size_mib, 512);
}

#[test]
fn kvm_unavailable_on_non_linux() {
    // On macOS/Windows, KVM should never be available
    if !cfg!(target_os = "linux") {
        assert!(!FirecrackerSandbox::kvm_available());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn firecracker_create_requires_kvm() {
    use worldcompute::sandbox::Sandbox;

    let tmp = std::env::temp_dir().join("wc-t030-fc-kvm");
    let _ = std::fs::remove_dir_all(&tmp);

    let mut sandbox = FirecrackerSandbox::new(tmp.clone());
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"test-workload").unwrap();

    let result = sandbox.create(&cid);
    if !FirecrackerSandbox::kvm_available() {
        assert!(result.is_err(), "create() should fail without KVM");
        assert!(
            result.unwrap_err().to_string().contains("KVM"),
            "Error should mention KVM"
        );
    }
    // If KVM is available, create() should succeed (rootfs is a placeholder)

    let _ = std::fs::remove_dir_all(&tmp);
}
