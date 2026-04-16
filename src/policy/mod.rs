//! Deterministic policy engine — the authoritative gate for all job admissions.
//!
//! Per FR-S040: wraps `validate_manifest()` as one step in a larger pipeline
//! that checks submitter identity, workload class approval, artifact registry,
//! resource limits, endpoint allowlists, data classification, quotas, and bans.
//!
//! The LLM advisory layer is non-authoritative (FR-S042). Disagreements between
//! the LLM and the deterministic engine are logged but never override the verdict.

pub mod decision;
pub mod engine;
pub mod rules;
