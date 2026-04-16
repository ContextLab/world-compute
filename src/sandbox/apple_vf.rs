//! Apple Virtualization.framework sandbox driver (macOS) per FR-010, FR-011.
//!
//! Uses macOS Virtualization.framework for VM-level isolation.
//! No GPU passthrough on macOS (blocked on Apple paravirtual GPU).

use crate::error::{ErrorCode, WcError};
use crate::sandbox::{Sandbox, SandboxCapability};
use crate::types::{Cid, DurationMs};

/// Apple Virtualization.framework sandbox state.
pub struct AppleVfSandbox {
    workload_cid: Option<Cid>,
    running: bool,
    frozen: bool,
    work_dir: std::path::PathBuf,
}

impl AppleVfSandbox {
    pub fn new(work_dir: std::path::PathBuf) -> Self {
        Self {
            workload_cid: None,
            running: false,
            frozen: false,
            work_dir,
        }
    }

    /// Check if Virtualization.framework is available.
    pub fn available() -> bool {
        cfg!(target_os = "macos")
    }
}

impl Sandbox for AppleVfSandbox {
    fn create(&mut self, workload_cid: &Cid) -> Result<(), WcError> {
        if !Self::available() {
            return Err(WcError::new(
                ErrorCode::SandboxUnavailable,
                "Apple Virtualization.framework requires macOS",
            ));
        }
        self.workload_cid = Some(*workload_cid);
        // TODO: Configure VZVirtualMachineConfiguration,
        // set up VZDiskImageStorageDeviceAttachment for rootfs,
        // configure network (NAT, no host bridge), memory, CPUs.
        tracing::info!(
            workload_cid = %workload_cid,
            "Apple VF sandbox created"
        );
        Ok(())
    }

    fn start(&mut self) -> Result<(), WcError> {
        // TODO: Start VZVirtualMachine, wait for guest agent.
        self.running = true;
        tracing::info!("Apple VF sandbox started");
        Ok(())
    }

    fn freeze(&mut self) -> Result<(), WcError> {
        // TODO: VZVirtualMachine.pause() — must complete within 10ms.
        self.frozen = true;
        tracing::info!("Apple VF sandbox frozen");
        Ok(())
    }

    fn checkpoint(&mut self, budget: DurationMs) -> Result<Cid, WcError> {
        // TODO: VZVirtualMachine.saveMachineStateTo, snapshot to CID.
        let _ = budget;
        Err(WcError::new(
            ErrorCode::Internal,
            "Apple VF checkpoint not yet implemented",
        ))
    }

    fn terminate(&mut self) -> Result<(), WcError> {
        // TODO: VZVirtualMachine.stop()
        self.running = false;
        self.frozen = false;
        tracing::info!("Apple VF sandbox terminated");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), WcError> {
        if self.work_dir.exists() {
            std::fs::remove_dir_all(&self.work_dir).map_err(|e| {
                WcError::new(ErrorCode::Internal, format!("Cleanup failed: {e}"))
            })?;
        }
        tracing::info!("Apple VF sandbox cleaned up");
        Ok(())
    }

    fn capability(&self) -> SandboxCapability {
        SandboxCapability::AppleVF
    }
}
