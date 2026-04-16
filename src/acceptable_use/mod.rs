//! Acceptable use module — policy enforcement per FR-080, FR-081.

use serde::{Deserialize, Serialize};

/// Acceptable use class for workloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcceptableUseClass {
    Scientific,
    PublicGoodMl,
    Rendering,
    Indexing,
    SelfImprovement,
    GeneralCompute,
}

/// Shard category for per-donor residency allowlist per FR-074.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShardCategory {
    Public,
    OpaqueEncrypted,
    EuResident,
    UsResident,
    UkResident,
    JpResident,
}
