//! GPU passthrough verification per FR-012.
//!
//! Checks singleton IOMMU group before exposing GPU to a guest.
//! The ACS-override patch is explicitly prohibited.

// Error types will be used when GPU check is fully implemented.
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

#[cfg(target_os = "linux")]
fn check_linux_gpu() -> GpuPassthroughResult {
    use std::fs;
    use std::path::Path;

    // Find NVIDIA/AMD GPU PCI devices
    let sysfs = Path::new("/sys/bus/pci/devices");
    if !sysfs.exists() {
        return GpuPassthroughResult {
            eligible: false,
            gpu_model: None,
            iommu_group: None,
            reason: "No sysfs PCI bus found".into(),
        };
    }

    // TODO: Enumerate PCI devices, find VGA controllers (class 0x030000),
    // check their IOMMU group membership count.
    // For now, return ineligible as a safe default.
    // Real implementation will:
    // 1. Read /sys/bus/pci/devices/*/class to find GPU
    // 2. Read /sys/bus/pci/devices/*/iommu_group to find group
    // 3. Count devices in that group — must be exactly 1 (singleton)
    // 4. Reject if ACS override patch is detected in dmesg

    GpuPassthroughResult {
        eligible: false,
        gpu_model: None,
        iommu_group: None,
        reason: "GPU passthrough check not yet fully implemented — defaulting to ineligible (safe)"
            .into(),
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
}
