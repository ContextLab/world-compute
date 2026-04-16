//! Preemption module — donor sovereignty enforcement per FR-040, FR-041.
//!
//! The preemption supervisor detects local user activity and freezes all
//! cluster workloads within 10ms (SIGSTOP), then checkpoints and releases
//! resources within 500ms. This is LOCAL-ONLY — no network call on the
//! critical preemption path.

pub mod supervisor;
pub mod triggers;
