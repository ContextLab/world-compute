//! GPU passthrough verification per FR-012.
//!
//! Checks singleton IOMMU group before exposing GPU to a guest.
//! The ACS-override patch is explicitly prohibited.

#[allow(unused_imports)]
use crate::error::{ErrorCode, WcError};

/// Result of GPU passthrough eligibility check.
#[derive(Debug, Clone)]
pub struct GpuPassthroughResult {
    pub eligible: bool,
    pub gpu_model: Option<String>,
    pub iommu_group: Option<u32>,
    pub reason: String,
}

/// A discovered GPU device on the PCI bus.
#[derive(Debug, Clone)]
pub struct GpuDevice {
    /// PCI device path (e.g. "0000:01:00.0").
    pub pci_address: String,
    /// Full sysfs path.
    pub sysfs_path: std::path::PathBuf,
}

/// Check if GPU passthrough is safe on this host.
/// Returns eligible=true only if the GPU is in a singleton IOMMU group.
pub fn check_gpu_passthrough() -> GpuPassthroughResult {
    #[cfg(target_os = "linux")]
    {
        check_linux_gpu()
    }
    #[cfg(not(target_os = "linux"))]
    {
        GpuPassthroughResult {
            eligible: false,
            gpu_model: None,
            iommu_group: None,
            reason: "GPU passthrough is only supported on Linux with IOMMU".into(),
        }
    }
}

/// Enumerate PCI devices and return those whose class starts with 0x0300 (VGA controllers).
///
/// On non-Linux platforms, returns an empty list.
pub fn enumerate_gpus() -> Vec<GpuDevice> {
    enumerate_gpus_at("/sys/bus/pci/devices")
}

/// Enumerate GPUs from a given sysfs-style base path (testable).
pub fn enumerate_gpus_at(base: &str) -> Vec<GpuDevice> {
    let sysfs = std::path::Path::new(base);
    if !sysfs.exists() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(sysfs) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut gpus = Vec::new();
    for entry in entries.flatten() {
        let class_file = entry.path().join("class");
        if let Ok(contents) = std::fs::read_to_string(&class_file) {
            let trimmed = contents.trim();
            // VGA compatible controller class is 0x030000 (or starts with 0x0300)
            if trimmed.starts_with("0x0300") {
                gpus.push(GpuDevice {
                    pci_address: entry.file_name().to_string_lossy().to_string(),
                    sysfs_path: entry.path(),
                });
            }
        }
    }
    gpus
}

/// Check whether the given PCI device is in a singleton IOMMU group.
///
/// Returns Ok(true) if the device is the only member of its IOMMU group (safe for passthrough).
/// Returns Ok(false) if there are other devices in the group (reject passthrough).
/// Returns Err if the IOMMU group cannot be read.
pub fn check_iommu_singleton(device_sysfs_path: &std::path::Path) -> Result<bool, WcError> {
    let iommu_devices = device_sysfs_path.join("iommu_group").join("devices");
    if !iommu_devices.exists() {
        return Err(WcError::new(
            ErrorCode::SandboxUnavailable,
            format!("No IOMMU group found for device {}", device_sysfs_path.display()),
        ));
    }

    let count = match std::fs::read_dir(&iommu_devices) {
        Ok(entries) => entries.count(),
        Err(e) => {
            return Err(WcError::new(
                ErrorCode::Internal,
                format!("Failed to read IOMMU group devices: {e}"),
            ));
        }
    };

    Ok(count == 1)
}

/// Detect unsafe ACS-override configurations.
///
/// Checks:
/// 1. `/sys/module/vfio/parameters/enable_unsafe_noiommu_mode` contains "Y"
/// 2. `/proc/cmdline` contains `pcie_acs_override`
///
/// Returns Ok(true) if ACS override is detected (should REJECT passthrough).
pub fn detect_acs_override() -> Result<bool, WcError> {
    detect_acs_override_at(
        "/sys/module/vfio/parameters/enable_unsafe_noiommu_mode",
        "/proc/cmdline",
    )
}

/// Testable version that accepts custom paths.
pub fn detect_acs_override_at(noiommu_path: &str, cmdline_path: &str) -> Result<bool, WcError> {
    // Check unsafe noiommu mode
    if let Ok(contents) = std::fs::read_to_string(noiommu_path) {
        if contents.trim() == "Y" {
            return Ok(true);
        }
    }

    // Check for pcie_acs_override in kernel command line
    if let Ok(contents) = std::fs::read_to_string(cmdline_path) {
        if contents.contains("pcie_acs_override") {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(target_os = "linux")]
fn check_linux_gpu() -> GpuPassthroughResult {
    // Check for ACS override first — reject if detected
    match detect_acs_override() {
        Ok(true) => {
            return GpuPassthroughResult {
                eligible: false,
                gpu_model: None,
                iommu_group: None,
                reason: "ACS override detected — unsafe IOMMU bypass, passthrough rejected".into(),
            };
        }
        Ok(false) => {}
        Err(_) => {
            // Cannot determine ACS state — proceed with caution
        }
    }

    let gpus = enumerate_gpus();
    if gpus.is_empty() {
        return GpuPassthroughResult {
            eligible: false,
            gpu_model: None,
            iommu_group: None,
            reason: "No VGA controllers (class 0x0300xx) found on PCI bus".into(),
        };
    }

    // Check the first GPU for singleton IOMMU group
    let gpu = &gpus[0];
    match check_iommu_singleton(&gpu.sysfs_path) {
        Ok(true) => GpuPassthroughResult {
            eligible: true,
            gpu_model: Some(gpu.pci_address.clone()),
            iommu_group: None,
            reason: "GPU is in singleton IOMMU group — passthrough safe".into(),
        },
        Ok(false) => GpuPassthroughResult {
            eligible: false,
            gpu_model: Some(gpu.pci_address.clone()),
            iommu_group: None,
            reason: "GPU shares IOMMU group with other devices — passthrough rejected".into(),
        },
        Err(e) => GpuPassthroughResult {
            eligible: false,
            gpu_model: Some(gpu.pci_address.clone()),
            iommu_group: None,
            reason: format!("IOMMU check failed: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_check_returns_result() {
        let result = check_gpu_passthrough();
        // On non-Linux (CI, macOS dev), should be ineligible
        if !cfg!(target_os = "linux") {
            assert!(!result.eligible);
        }
    }

    #[test]
    fn enumerate_gpus_returns_vec() {
        // May be empty on CI / non-Linux, but should not panic
        let gpus = enumerate_gpus();
        let _ = gpus.len();
    }

    #[test]
    fn enumerate_gpus_at_nonexistent_returns_empty() {
        let gpus = enumerate_gpus_at("/nonexistent/path/does/not/exist");
        assert!(gpus.is_empty());
    }

    #[test]
    fn enumerate_gpus_finds_vga_class() {
        let tmp = std::env::temp_dir().join("wc-test-gpu-enum");
        let _ = std::fs::remove_dir_all(&tmp);
        let dev_path = tmp.join("0000:01:00.0");
        std::fs::create_dir_all(&dev_path).unwrap();
        std::fs::write(dev_path.join("class"), "0x030000\n").unwrap();

        // Non-GPU device
        let other = tmp.join("0000:00:1f.0");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("class"), "0x060100\n").unwrap();

        let gpus = enumerate_gpus_at(tmp.to_str().unwrap());
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].pci_address, "0000:01:00.0");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn iommu_singleton_with_one_device() {
        let tmp = std::env::temp_dir().join("wc-test-iommu-single");
        let _ = std::fs::remove_dir_all(&tmp);
        let dev = tmp.join("0000:01:00.0");
        let iommu_devs = dev.join("iommu_group").join("devices");
        std::fs::create_dir_all(&iommu_devs).unwrap();
        // One device in the group
        std::fs::create_dir(iommu_devs.join("0000:01:00.0")).unwrap();

        assert!(check_iommu_singleton(&dev).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn iommu_non_singleton_rejects() {
        let tmp = std::env::temp_dir().join("wc-test-iommu-multi");
        let _ = std::fs::remove_dir_all(&tmp);
        let dev = tmp.join("0000:01:00.0");
        let iommu_devs = dev.join("iommu_group").join("devices");
        std::fs::create_dir_all(&iommu_devs).unwrap();
        std::fs::create_dir(iommu_devs.join("0000:01:00.0")).unwrap();
        std::fs::create_dir(iommu_devs.join("0000:01:00.1")).unwrap();

        assert!(!check_iommu_singleton(&dev).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn iommu_missing_group_returns_error() {
        let tmp = std::env::temp_dir().join("wc-test-iommu-missing");
        let _ = std::fs::remove_dir_all(&tmp);
        let dev = tmp.join("0000:02:00.0");
        std::fs::create_dir_all(&dev).unwrap();
        // No iommu_group directory
        assert!(check_iommu_singleton(&dev).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn acs_override_not_detected_on_missing_files() {
        let result = detect_acs_override_at("/nonexistent/noiommu_mode", "/nonexistent/cmdline");
        assert!(!result.unwrap());
    }

    #[test]
    fn acs_override_detected_via_noiommu() {
        let tmp = std::env::temp_dir().join("wc-test-acs-noiommu");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let noiommu = tmp.join("enable_unsafe_noiommu_mode");
        std::fs::write(&noiommu, "Y\n").unwrap();

        let result = detect_acs_override_at(noiommu.to_str().unwrap(), "/nonexistent/cmdline");
        assert!(result.unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn acs_override_detected_via_cmdline() {
        let tmp = std::env::temp_dir().join("wc-test-acs-cmdline");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cmdline = tmp.join("cmdline");
        std::fs::write(&cmdline, "root=/dev/sda1 pcie_acs_override=downstream\n").unwrap();

        let result = detect_acs_override_at("/nonexistent/noiommu_mode", cmdline.to_str().unwrap());
        assert!(result.unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
