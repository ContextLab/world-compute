//! Scheduler module — job model, priority, placement, broker, coordinator.

pub mod job;
pub mod manifest;
pub mod priority;

use serde::{Deserialize, Serialize};

/// Job category label for accounting (NOT rigid scheduling order).
/// Scheduling uses the continuous multi-factor priority score per FR-032.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobCategory {
    DonorRedemption,
    PaidSponsored,
    PublicGood,
    SelfImprovement,
}

/// Confidentiality level for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfidentialityLevel {
    /// Plaintext — public data, any trust tier.
    Public,
    /// Encrypted bundle with TPM-agent-attested key release.
    ConfidentialMedium,
    /// SEV-SNP/TDX/H100-CC guest-measurement key wrapping. T3+ only.
    ConfidentialHigh,
}

/// Verification method for a task's results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// R=3 replicated execution with canonical-hash quorum (default).
    ReplicatedQuorum,
    /// TEE-attested single execution (T3+ nodes only).
    TeeAttested,
    /// Custom replication factor.
    CustomReplicas(u32),
}

/// Workload format type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadType {
    /// OCI container image (CID-addressed).
    OciContainer,
    /// WASM module (CID-addressed).
    WasmModule,
}

/// Preemption class — how a workload handles being preempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreemptClass {
    /// Can be frozen (SIGSTOP) and resumed in-place.
    Yieldable,
    /// Can be checkpointed and resumed on another node.
    Checkpointable,
    /// Must be restarted from scratch on preemption.
    Restartable,
}

/// Resource envelope for a task or node capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEnvelope {
    pub cpu_millicores: u64,
    pub ram_bytes: u64,
    pub gpu_class: Option<u32>,
    pub gpu_vram_bytes: u64,
    pub scratch_bytes: u64,
    pub network_egress_bytes: u64,
    pub walltime_budget_ms: u64,
}
