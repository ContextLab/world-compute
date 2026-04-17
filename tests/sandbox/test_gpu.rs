//! T062: GPU passthrough verification tests.

use worldcompute::sandbox::gpu;

#[test]
fn check_linux_gpu_returns_vec() {
    let gpus = gpu::enumerate_gpus();
    // May be empty on CI / non-Linux, but should not panic
    let _ = gpus.len();
}

#[test]
fn enumerate_gpus_at_fake_sysfs() {
    let tmp = std::env::temp_dir().join("wc-t062-gpu-enum");
    let _ = std::fs::remove_dir_all(&tmp);

    // Create a fake VGA device
    let vga = tmp.join("0000:03:00.0");
    std::fs::create_dir_all(&vga).unwrap();
    std::fs::write(vga.join("class"), "0x030000\n").unwrap();

    // Create a non-GPU device
    let bridge = tmp.join("0000:00:1f.0");
    std::fs::create_dir_all(&bridge).unwrap();
    std::fs::write(bridge.join("class"), "0x060100\n").unwrap();

    let gpus = gpu::enumerate_gpus_at(tmp.to_str().unwrap());
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].pci_address, "0000:03:00.0");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn iommu_singleton_group_allows_passthrough() {
    let tmp = std::env::temp_dir().join("wc-t062-iommu-single");
    let _ = std::fs::remove_dir_all(&tmp);

    let dev = tmp.join("0000:03:00.0");
    let iommu_devs = dev.join("iommu_group").join("devices");
    std::fs::create_dir_all(&iommu_devs).unwrap();
    std::fs::create_dir_all(iommu_devs.join("0000:03:00.0")).unwrap();

    assert!(gpu::check_iommu_singleton(&dev).unwrap());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn iommu_shared_group_rejects_passthrough() {
    let tmp = std::env::temp_dir().join("wc-t062-iommu-shared");
    let _ = std::fs::remove_dir_all(&tmp);

    let dev = tmp.join("0000:03:00.0");
    let iommu_devs = dev.join("iommu_group").join("devices");
    std::fs::create_dir_all(&iommu_devs).unwrap();
    std::fs::create_dir(iommu_devs.join("0000:03:00.0")).unwrap();
    std::fs::create_dir(iommu_devs.join("0000:03:00.1")).unwrap();

    assert!(!gpu::check_iommu_singleton(&dev).unwrap());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn acs_override_detected_noiommu_mode() {
    let tmp = std::env::temp_dir().join("wc-t062-acs-noiommu");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let noiommu = tmp.join("noiommu_mode");
    std::fs::write(&noiommu, "Y\n").unwrap();

    let result = gpu::detect_acs_override_at(noiommu.to_str().unwrap(), "/nonexistent/cmdline");
    assert!(result.unwrap());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn acs_override_detected_cmdline() {
    let tmp = std::env::temp_dir().join("wc-t062-acs-cmdline");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let cmdline = tmp.join("cmdline");
    std::fs::write(&cmdline, "root=/dev/sda1 pcie_acs_override=downstream,multifunction\n")
        .unwrap();

    let result = gpu::detect_acs_override_at("/nonexistent/noiommu", cmdline.to_str().unwrap());
    assert!(result.unwrap());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn acs_override_not_detected_clean_system() {
    let tmp = std::env::temp_dir().join("wc-t062-acs-clean");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let noiommu = tmp.join("noiommu_mode");
    std::fs::write(&noiommu, "N\n").unwrap();

    let cmdline = tmp.join("cmdline");
    std::fs::write(&cmdline, "root=/dev/sda1 quiet splash\n").unwrap();

    let result = gpu::detect_acs_override_at(noiommu.to_str().unwrap(), cmdline.to_str().unwrap());
    assert!(!result.unwrap());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn gpu_passthrough_check_returns_result() {
    let result = gpu::check_gpu_passthrough();
    if !cfg!(target_os = "linux") {
        assert!(!result.eligible);
    }
}
