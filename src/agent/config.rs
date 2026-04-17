//! Agent configuration — load from file, env vars, CLI overrides (T027).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent configuration loaded from file and overridable by env/CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Scoped working directory for sandbox and data.
    pub work_dir: PathBuf,
    /// Maximum CPU percentage to donate (0-100).
    pub cpu_cap_percent: u32,
    /// Maximum storage in bytes for the local CID store.
    pub storage_cap_bytes: u64,
    /// OpenTelemetry collector endpoint.
    pub otel_endpoint: Option<String>,
    /// Ed25519 key file path.
    pub key_path: PathBuf,
    /// Idle detection sensitivity in milliseconds.
    pub idle_threshold_ms: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            work_dir: std::env::temp_dir().join("worldcompute"),
            cpu_cap_percent: 80,
            storage_cap_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            otel_endpoint: None,
            key_path: PathBuf::from("~/.worldcompute/key"),
            idle_threshold_ms: 2000,
        }
    }
}
