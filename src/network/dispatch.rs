//! Distributed job dispatch protocols — request-response RPCs over libp2p.
//!
//! Two protocols:
//!
//! 1. **TaskOffer** (`/worldcompute/offer/1.0.0`): a broker sends a candidate
//!    executor a task summary (CPU/memory/GPU requirements). The executor
//!    replies whether it has capacity and its current load. Lightweight —
//!    the task manifest itself is NOT sent at this stage.
//!
//! 2. **TaskDispatch** (`/worldcompute/dispatch/1.0.0`): after selecting an
//!    executor, the broker sends the full JobManifest. The executor runs the
//!    workload inside its sandbox and replies with the result (or an error).
//!
//! Both protocols use CBOR for efficient, schema-evolvable serialization.

use libp2p::request_response::{self, ProtocolSupport};
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::scheduler::manifest::JobManifest;

/// Protocol identifier for TaskOffer (lightweight capacity probe).
pub const PROTOCOL_TASK_OFFER: &str = "/worldcompute/offer/1.0.0";

/// Protocol identifier for TaskDispatch (full job manifest + result).
pub const PROTOCOL_TASK_DISPATCH: &str = "/worldcompute/dispatch/1.0.0";

// ============================================================================
// TaskOffer protocol
// ============================================================================

/// A lightweight task description for capacity negotiation.
/// Sent broker → candidate executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOffer {
    pub task_id: String,
    pub manifest_cid: String,
    /// Minimum CPU cores required.
    pub min_cpu_cores: u32,
    /// Minimum memory in megabytes.
    pub min_memory_mb: u64,
    /// Whether GPU is required.
    pub needs_gpu: bool,
    /// Maximum wallclock in milliseconds.
    pub max_wallclock_ms: u64,
}

/// Executor's response to a TaskOffer.
/// Sent candidate executor → broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOfferResponse {
    pub task_id: String,
    /// True if the executor accepts and has capacity.
    pub accepted: bool,
    /// Current load as a fraction 0.0 to 1.0 (0 = idle, 1 = saturated).
    pub load: f32,
    /// Optional reason for rejection.
    pub reason: Option<String>,
}

// ============================================================================
// TaskDispatch protocol
// ============================================================================

/// Full job dispatch request.
/// Sent broker → selected executor after capacity negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDispatchRequest {
    pub task_id: String,
    pub manifest: JobManifest,
    /// Input data (inlined for small inputs; CIDs for large inputs fetched from data plane).
    pub inline_inputs: Vec<(String, Vec<u8>)>,
}

/// Result of executing a job, returned to the broker.
/// Sent executor → broker after workload completes (or fails).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDispatchResponse {
    pub task_id: String,
    pub status: TaskStatus,
    /// Output bytes (for small results) or empty with output_cid set.
    pub output: Vec<u8>,
    /// Optional CID of output stored in data plane (for large results).
    pub output_cid: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if status is Failed.
    pub error: Option<String>,
}

/// Outcome of a dispatched task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Succeeded,
    Failed,
    Timeout,
    Preempted,
}

// ============================================================================
// Behaviour construction
// ============================================================================

/// Build the TaskOffer request-response behaviour.
pub fn build_offer_behaviour() -> request_response::cbor::Behaviour<TaskOffer, TaskOfferResponse> {
    let protocols =
        std::iter::once((StreamProtocol::new(PROTOCOL_TASK_OFFER), ProtocolSupport::Full));
    let config = request_response::Config::default()
        .with_request_timeout(Duration::from_secs(10))
        .with_max_concurrent_streams(100);
    request_response::cbor::Behaviour::new(protocols, config)
}

/// Build the TaskDispatch request-response behaviour.
pub fn build_dispatch_behaviour(
) -> request_response::cbor::Behaviour<TaskDispatchRequest, TaskDispatchResponse> {
    let protocols =
        std::iter::once((StreamProtocol::new(PROTOCOL_TASK_DISPATCH), ProtocolSupport::Full));
    // Dispatch has a much longer timeout — the executor is actually running the job.
    let config = request_response::Config::default()
        .with_request_timeout(Duration::from_secs(600))
        .with_max_concurrent_streams(20);
    request_response::cbor::Behaviour::new(protocols, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_names_are_versioned() {
        assert!(PROTOCOL_TASK_OFFER.starts_with("/worldcompute/"));
        assert!(PROTOCOL_TASK_OFFER.contains("1.0.0"));
        assert!(PROTOCOL_TASK_DISPATCH.starts_with("/worldcompute/"));
        assert!(PROTOCOL_TASK_DISPATCH.contains("1.0.0"));
    }

    #[test]
    fn task_offer_serializes() {
        let offer = TaskOffer {
            task_id: "task-001".into(),
            manifest_cid: "bafybeia".into(),
            min_cpu_cores: 2,
            min_memory_mb: 1024,
            needs_gpu: false,
            max_wallclock_ms: 60_000,
        };
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&offer, &mut bytes).expect("serialize");
        assert!(!bytes.is_empty());
        let decoded: TaskOffer = ciborium::de::from_reader(&bytes[..]).expect("deserialize");
        assert_eq!(decoded.task_id, offer.task_id);
        assert_eq!(decoded.needs_gpu, offer.needs_gpu);
    }

    #[test]
    fn task_offer_response_rejection() {
        let resp = TaskOfferResponse {
            task_id: "task-001".into(),
            accepted: false,
            load: 0.95,
            reason: Some("saturated".into()),
        };
        assert!(!resp.accepted);
        assert!((resp.load - 0.95).abs() < 1e-6);
    }

    #[test]
    fn task_status_all_variants() {
        assert_eq!(TaskStatus::Succeeded, TaskStatus::Succeeded);
        assert_ne!(TaskStatus::Succeeded, TaskStatus::Failed);
        assert_ne!(TaskStatus::Failed, TaskStatus::Timeout);
        assert_ne!(TaskStatus::Timeout, TaskStatus::Preempted);
    }

    #[test]
    fn build_behaviours_dont_panic() {
        let _offer = build_offer_behaviour();
        let _dispatch = build_dispatch_behaviour();
    }

    #[test]
    fn dispatch_response_fields() {
        let resp = TaskDispatchResponse {
            task_id: "task-002".into(),
            status: TaskStatus::Succeeded,
            output: b"hello world".to_vec(),
            output_cid: None,
            duration_ms: 1234,
            error: None,
        };
        assert_eq!(resp.status, TaskStatus::Succeeded);
        assert_eq!(resp.output, b"hello world");
        assert_eq!(resp.duration_ms, 1234);
    }
}
