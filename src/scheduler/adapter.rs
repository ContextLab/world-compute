//! ComputeAdapter trait — uniform interface for Slurm, K8s, and cloud backends.

use crate::{error::WcResult, scheduler::ResourceEnvelope, types::Cid};

/// Status of a task as reported by an adapter backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterTaskStatus {
    /// Task has been accepted by the backend but not yet started.
    Pending,
    /// Task is actively executing.
    Running,
    /// Task finished successfully; contains the output CID.
    Completed(Cid),
    /// Task failed; contains a human-readable reason.
    Failed(String),
}

/// Uniform interface every compute backend must implement.
///
/// Adapters are responsible for translating World Compute abstractions
/// (task IDs, workload CIDs, resource envelopes) into backend-specific
/// operations (Slurm job scripts, K8s CRDs, cloud instance APIs).
pub trait ComputeAdapter {
    /// Register this adapter with the World Compute coordinator.
    fn register(&mut self) -> WcResult<()>;

    /// Deregister this adapter, draining in-flight tasks gracefully.
    fn deregister(&mut self) -> WcResult<()>;

    /// Submit a task to the backend.
    ///
    /// * `task_id` — stable UUID string for this task
    /// * `workload_cid` — CIDv1 of the OCI/WASM workload bundle
    /// * `resources` — resource envelope the task is entitled to
    fn submit_task(
        &mut self,
        task_id: &str,
        workload_cid: Cid,
        resources: ResourceEnvelope,
    ) -> WcResult<()>;

    /// Poll the current status of a previously submitted task.
    fn get_status(&self, task_id: &str) -> WcResult<AdapterTaskStatus>;

    /// Report the current available capacity on this backend.
    fn get_capacity(&self) -> ResourceEnvelope;

    /// Perform a liveness check against the backend control plane.
    fn health_check(&self) -> WcResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_task_status_pending_variant() {
        let s = AdapterTaskStatus::Pending;
        assert_eq!(s, AdapterTaskStatus::Pending);
    }

    #[test]
    fn adapter_task_status_running_variant() {
        let s = AdapterTaskStatus::Running;
        assert_eq!(s, AdapterTaskStatus::Running);
    }

    #[test]
    fn adapter_task_status_failed_variant() {
        let s = AdapterTaskStatus::Failed("out of memory".to_string());
        assert!(matches!(s, AdapterTaskStatus::Failed(_)));
        if let AdapterTaskStatus::Failed(msg) = s {
            assert_eq!(msg, "out of memory");
        }
    }

    #[test]
    fn adapter_task_status_completed_variant() {
        // Build a well-formed CID for the test.
        use cid::multihash::Multihash;
        let mh = Multihash::wrap(0x12, &[0u8; 32]).expect("multihash");
        let cid = Cid::new_v1(0x55, mh);
        let s = AdapterTaskStatus::Completed(cid);
        assert!(matches!(s, AdapterTaskStatus::Completed(_)));
    }
}
