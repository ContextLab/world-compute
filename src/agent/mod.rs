//! Agent module — per-host background process lifecycle.

pub mod config;
pub mod donor;
pub mod identity;
pub mod mesh_llm;
pub mod node;

use serde::{Deserialize, Serialize};

/// Agent lifecycle states per data-model §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    Enrolling,
    Idle,
    Working,
    Paused,
    Withdrawing,
}
