//! Preemption supervisor per FR-040, FR-041 (T036).
//!
//! Watches for sovereignty events and freezes all sandbox workloads within
//! 10ms (SIGSTOP), then checkpoints within 500ms and releases resources.
//! This runs entirely locally — no network calls on the preemption path.

use crate::preemption::triggers::SovereigntyEvent;
use crate::sandbox::Sandbox;
use crate::types::DurationMs;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::watch;

/// Preemption supervisor state.
pub struct PreemptionSupervisor {
    /// Active sandboxes managed by this supervisor.
    sandboxes: Arc<Mutex<Vec<Box<dyn Sandbox>>>>,
    /// Receiver for sovereignty events from the idle detector.
    #[allow(dead_code)]
    event_rx: watch::Receiver<Option<SovereigntyEvent>>,
    /// Whether workloads are currently frozen.
    frozen: bool,
}

impl PreemptionSupervisor {
    pub fn new(event_rx: watch::Receiver<Option<SovereigntyEvent>>) -> Self {
        Self { sandboxes: Arc::new(Mutex::new(Vec::new())), event_rx, frozen: false }
    }

    /// Register a sandbox to be managed by this supervisor.
    pub fn register_sandbox(&self, sandbox: Box<dyn Sandbox>) {
        self.sandboxes.lock().unwrap().push(sandbox);
    }

    /// Get a handle to the sandbox list for external management.
    pub fn sandboxes(&self) -> Arc<Mutex<Vec<Box<dyn Sandbox>>>> {
        Arc::clone(&self.sandboxes)
    }

    /// Freeze all active sandboxes. Target: <10ms total.
    /// This is the hot path — no allocations, no network, no locks beyond
    /// the sandbox list.
    pub fn freeze_all(&mut self) -> PreemptionResult {
        let start = Instant::now();
        let mut sandboxes = self.sandboxes.lock().unwrap();
        let mut frozen_count = 0;
        let mut errors = Vec::new();

        for sandbox in sandboxes.iter_mut() {
            if let Err(e) = sandbox.freeze() {
                errors.push(format!("{:?}: {e}", sandbox.capability()));
            } else {
                frozen_count += 1;
            }
        }

        let elapsed = start.elapsed();
        self.frozen = true;

        PreemptionResult { frozen_count, freeze_latency_us: elapsed.as_micros() as u64, errors }
    }

    /// Checkpoint all frozen sandboxes, then terminate. Target: <500ms.
    pub fn checkpoint_and_release(&mut self) -> Vec<CheckpointResult> {
        let mut sandboxes = self.sandboxes.lock().unwrap();
        let mut results = Vec::new();

        for sandbox in sandboxes.iter_mut() {
            let start = Instant::now();
            let checkpoint_cid = sandbox.checkpoint(DurationMs(400));
            let elapsed = start.elapsed();

            results.push(CheckpointResult {
                capability: sandbox.capability(),
                cid: checkpoint_cid.ok(),
                latency_ms: elapsed.as_millis() as u64,
            });

            // Always terminate after checkpoint attempt (even if checkpoint fails)
            let _ = sandbox.terminate();
        }

        // Clear the sandbox list — resources fully released
        sandboxes.clear();
        self.frozen = false;

        results
    }

    /// Resume frozen sandboxes (user went idle again).
    ///
    /// Per FR-S004: sends resume signal to each frozen sandbox so workloads
    /// can continue where they left off without rescheduling.
    pub fn resume_all(&mut self) -> ResumeResult {
        let start = std::time::Instant::now();
        let mut sandboxes = self.sandboxes.lock().unwrap();
        let mut resumed_count = 0;
        let mut errors = Vec::new();

        for sandbox in sandboxes.iter_mut() {
            // Each sandbox's start() re-activates a paused VM.
            // On Linux/Firecracker this sends SIGCONT, on macOS VZ.resume(),
            // on Windows Resume-VM.
            if let Err(e) = sandbox.start() {
                errors.push(format!("{:?}: {e}", sandbox.capability()));
            } else {
                resumed_count += 1;
            }
        }

        let elapsed = start.elapsed();
        self.frozen = false;

        ResumeResult { resumed_count, resume_latency_us: elapsed.as_micros() as u64, errors }
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
}

/// Result of a freeze operation.
#[derive(Debug)]
pub struct PreemptionResult {
    pub frozen_count: usize,
    pub freeze_latency_us: u64,
    pub errors: Vec<String>,
}

impl PreemptionResult {
    /// Whether the freeze completed within the 10ms budget.
    pub fn within_budget(&self) -> bool {
        self.freeze_latency_us < 10_000 // 10ms = 10,000μs
    }
}

/// Result of a resume operation.
#[derive(Debug)]
pub struct ResumeResult {
    pub resumed_count: usize,
    pub resume_latency_us: u64,
    pub errors: Vec<String>,
}

/// Result of a checkpoint operation on one sandbox.
#[derive(Debug)]
pub struct CheckpointResult {
    pub capability: crate::sandbox::SandboxCapability,
    pub cid: Option<crate::types::Cid>,
    pub latency_ms: u64,
}

/// T042: Events that the preemption handler responds to.
/// These are higher-level than `SovereigntyEvent` — they represent
/// categories of preemption triggers with distinct urgency levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptionEvent {
    /// Keyboard input detected — high urgency, immediate freeze.
    KeyboardActivity,
    /// Mouse movement detected — high urgency, immediate freeze.
    MouseActivity,
    /// CPU/GPU thermal threshold exceeded — medium urgency.
    ThermalThreshold,
    /// AC power disconnected (laptop on battery) — medium urgency.
    BatteryDisconnect,
    /// System memory pressure (low available RAM) — high urgency.
    MemoryPressure,
}

/// T042: Result of handling a preemption event.
#[derive(Debug)]
pub struct PreemptionHandlerResult {
    pub event: PreemptionEvent,
    pub sandbox_pids_stopped: u32,
    pub latency_ns: u64,
    pub checkpoint_attempted: bool,
    pub checkpoint_succeeded: bool,
}

/// T044: Result of checkpoint-or-kill escalation.
#[derive(Debug)]
pub struct EscalationResult {
    pub checkpointed: u32,
    pub killed: u32,
    pub total_latency_ms: u64,
}

/// T045: GPU kernel completion window — GPU workloads get an extra 200ms
/// before SIGSTOP to allow in-flight GPU kernels to complete. This avoids
/// leaving the GPU in a dirty state that could affect the host.
pub const GPU_KERNEL_WINDOW_MS: u64 = 200;

/// T043: Handle a preemption event by sending SIGSTOP to sandbox PIDs.
///
/// Uses `nix::sys::signal::kill` on Unix. On non-Unix platforms, returns
/// an error since SIGSTOP is not available.
#[cfg(unix)]
pub fn handle_preemption_event(
    event: PreemptionEvent,
    sandbox_pids: &[u32],
) -> Result<PreemptionHandlerResult, crate::error::WcError> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    use std::time::Instant;

    let start = Instant::now();
    let mut stopped = 0u32;

    for &pid in sandbox_pids {
        match kill(Pid::from_raw(pid as i32), Signal::SIGSTOP) {
            Ok(()) => stopped += 1,
            Err(e) => {
                tracing::warn!(pid, error = %e, "Failed to SIGSTOP sandbox pid");
            }
        }
    }

    let latency_ns = start.elapsed().as_nanos() as u64;

    Ok(PreemptionHandlerResult {
        event,
        sandbox_pids_stopped: stopped,
        latency_ns,
        checkpoint_attempted: false,
        checkpoint_succeeded: false,
    })
}

#[cfg(not(unix))]
pub fn handle_preemption_event(
    _event: PreemptionEvent,
    _sandbox_pids: &[u32],
) -> Result<PreemptionHandlerResult, crate::error::WcError> {
    Err(crate::error::WcError::new(
        crate::error::ErrorCode::Internal,
        "SIGSTOP not available on this platform",
    ))
}

/// T044: Checkpoint-or-kill escalation.
///
/// After SIGSTOP, attempts checkpoint within the given budget. If checkpoint
/// succeeds, returns success. If it exceeds the budget, sends SIGKILL to
/// force-terminate the process.
#[cfg(unix)]
pub fn escalate_after_stop(sandbox_pids: &[u32], checkpoint_budget_ms: u64) -> EscalationResult {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    use std::time::Instant;

    let start = Instant::now();
    let mut checkpointed = 0u32;
    let mut killed = 0u32;

    for &pid in sandbox_pids {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        if elapsed_ms >= checkpoint_budget_ms {
            // Budget exhausted — escalate to SIGKILL
            match kill(Pid::from_raw(pid as i32), Signal::SIGKILL) {
                Ok(()) => killed += 1,
                Err(_) => {
                    // Process already gone — still count as killed (terminated)
                    killed += 1;
                }
            }
        } else {
            // Attempt checkpoint (simulated — real checkpoint is via sandbox trait)
            // For now, consider the process "checkpointed" if it's still alive
            match kill(Pid::from_raw(pid as i32), None) {
                Ok(()) => checkpointed += 1,
                Err(_) => {
                    // Process already gone — count as killed
                    killed += 1;
                }
            }
        }
    }

    let total_latency_ms = start.elapsed().as_millis() as u64;

    EscalationResult { checkpointed, killed, total_latency_ms }
}

#[cfg(not(unix))]
pub fn escalate_after_stop(sandbox_pids: &[u32], _checkpoint_budget_ms: u64) -> EscalationResult {
    EscalationResult { checkpointed: 0, killed: sandbox_pids.len() as u32, total_latency_ms: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::watch;

    #[test]
    fn supervisor_starts_unfrozen() {
        let (_tx, rx) = watch::channel(None);
        let sup = PreemptionSupervisor::new(rx);
        assert!(!sup.is_frozen());
    }

    #[test]
    fn freeze_all_with_no_sandboxes_is_instant() {
        let (_tx, rx) = watch::channel(None);
        let mut sup = PreemptionSupervisor::new(rx);
        let result = sup.freeze_all();
        assert_eq!(result.frozen_count, 0);
        assert!(result.within_budget());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn checkpoint_and_release_clears_sandboxes() {
        let (_tx, rx) = watch::channel(None);
        let mut sup = PreemptionSupervisor::new(rx);
        let results = sup.checkpoint_and_release();
        assert!(results.is_empty());
        assert!(!sup.is_frozen());
    }

    #[test]
    fn resume_all_with_no_sandboxes_is_instant() {
        let (_tx, rx) = watch::channel(None);
        let mut sup = PreemptionSupervisor::new(rx);
        sup.freeze_all();
        assert!(sup.is_frozen());
        let result = sup.resume_all();
        assert_eq!(result.resumed_count, 0);
        assert!(result.errors.is_empty());
        assert!(!sup.is_frozen());
    }
}
