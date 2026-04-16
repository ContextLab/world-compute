//! Sandbox module — VM-level workload isolation.
//!
//! Per FR-010: all workloads MUST execute inside a hypervisor- or VM-level
//! sandbox. Process-only sandboxes are NOT sufficient.

pub mod apple_vf;
pub mod firecracker;
pub mod gpu;
pub mod hyperv;
pub mod wasm;

use crate::types::{Cid, DurationMs};
use serde::{Deserialize, Serialize};

/// Platform the agent is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Browser,
    Mobile,
}

impl Platform {
    /// Detect the current platform at compile time.
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        return Self::Linux;
        #[cfg(target_os = "macos")]
        return Self::MacOS;
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Self::Browser;
    }
}

/// Sandbox capability available on this platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxCapability {
    /// Firecracker microVM (Linux KVM)
    Firecracker,
    /// Apple Virtualization.framework (macOS)
    AppleVF,
    /// Microsoft Hyper-V (Windows Pro)
    HyperV,
    /// WSL2 utility VM (Windows Home with WHPX)
    Wsl2,
    /// WASM runtime only (Tier 3 / browser / low-trust)
    WasmOnly,
}

/// Lifecycle trait that all sandbox drivers must implement.
pub trait Sandbox: Send + Sync {
    /// Create the sandbox environment.
    fn create(&mut self, workload_cid: &Cid) -> Result<(), crate::error::WcError>;
    /// Start executing the workload inside the sandbox.
    fn start(&mut self) -> Result<(), crate::error::WcError>;
    /// Freeze the workload (SIGSTOP equivalent). Must complete within 10ms.
    fn freeze(&mut self) -> Result<(), crate::error::WcError>;
    /// Checkpoint current state to a CID within the given budget.
    fn checkpoint(&mut self, budget: DurationMs) -> Result<Cid, crate::error::WcError>;
    /// Terminate the workload and release all resources.
    fn terminate(&mut self) -> Result<(), crate::error::WcError>;
    /// Clean up all sandbox artifacts. Must leave no host residue.
    fn cleanup(&mut self) -> Result<(), crate::error::WcError>;
    /// Check if GPU passthrough is available and safe (singleton IOMMU group).
    fn gpu_available(&self) -> bool {
        false
    }
    /// Return the sandbox capability type.
    fn capability(&self) -> SandboxCapability;
}

/// Factory: detect platform and return the appropriate sandbox capability.
pub fn detect_capability() -> SandboxCapability {
    match Platform::detect() {
        Platform::Linux => SandboxCapability::Firecracker,
        Platform::MacOS => SandboxCapability::AppleVF,
        Platform::Windows => SandboxCapability::HyperV,
        Platform::Browser | Platform::Mobile => SandboxCapability::WasmOnly,
    }
}
