//! Hyper-V sandbox driver (Windows) per FR-010, FR-011.
//!
//! Uses Hyper-V on Windows Pro, falls back to WSL2/WHPX on Windows Home.

use crate::error::{ErrorCode, WcError};
use crate::sandbox::{Sandbox, SandboxCapability};
use crate::types::{Cid, DurationMs};

/// Hyper-V sandbox state.
pub struct HyperVSandbox {
    workload_cid: Option<Cid>,
    running: bool,
    frozen: bool,
    work_dir: std::path::PathBuf,
    is_wsl2_fallback: bool,
}

impl HyperVSandbox {
    pub fn new(work_dir: std::path::PathBuf) -> Self {
        Self {
            workload_cid: None,
            running: false,
            frozen: false,
            work_dir,
            is_wsl2_fallback: false,
        }
    }

    /// Detect Hyper-V availability; fall back to WSL2/WHPX on Home edition.
    pub fn detect() -> Option<SandboxCapability> {
        #[cfg(target_os = "windows")]
        {
            // TODO: Check for Hyper-V via WMI; fall back to WSL2 if unavailable.
            Some(SandboxCapability::HyperV)
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

impl Sandbox for HyperVSandbox {
    fn create(&mut self, workload_cid: &Cid) -> Result<(), WcError> {
        if Self::detect().is_none() {
            return Err(WcError::new(ErrorCode::SandboxUnavailable, "Hyper-V requires Windows"));
        }
        self.workload_cid = Some(*workload_cid);
        // TODO: Create Hyper-V VM via COM/WMI API or windows-rs,
        // configure isolated virtual switch, attach VHD.
        tracing::info!(workload_cid = %workload_cid, "Hyper-V sandbox created");
        Ok(())
    }

    fn start(&mut self) -> Result<(), WcError> {
        self.running = true;
        tracing::info!("Hyper-V sandbox started");
        Ok(())
    }

    fn freeze(&mut self) -> Result<(), WcError> {
        // TODO: Hyper-V VM pause — must complete within 10ms.
        self.frozen = true;
        tracing::info!("Hyper-V sandbox frozen");
        Ok(())
    }

    fn checkpoint(&mut self, budget: DurationMs) -> Result<Cid, WcError> {
        let _ = budget;
        Err(WcError::new(ErrorCode::Internal, "Hyper-V checkpoint not yet implemented"))
    }

    fn terminate(&mut self) -> Result<(), WcError> {
        self.running = false;
        self.frozen = false;
        tracing::info!("Hyper-V sandbox terminated");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), WcError> {
        if self.work_dir.exists() {
            std::fs::remove_dir_all(&self.work_dir)
                .map_err(|e| WcError::new(ErrorCode::Internal, format!("Cleanup failed: {e}")))?;
        }
        tracing::info!("Hyper-V sandbox cleaned up");
        Ok(())
    }

    fn gpu_available(&self) -> bool {
        // TODO: CUDA via GPU-P in WSL2 — check at runtime.
        false
    }

    fn capability(&self) -> SandboxCapability {
        if self.is_wsl2_fallback {
            SandboxCapability::Wsl2
        } else {
            SandboxCapability::HyperV
        }
    }
}
