//! Agent module — per-host background process lifecycle.

pub mod build_info;
pub mod config;
pub mod donor;
pub mod identity;
pub mod lifecycle;
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

// T039: The heartbeat loop will be wired as a tokio task in the async runtime.
// When the agent starts, `run_heartbeat_loop` is spawned to periodically call
// `AgentInstance::heartbeat()` and publish the payload over gossipsub.

/// Run the heartbeat loop as a tokio task. Calls `heartbeat()` every
/// `interval_secs` seconds and publishes the payload to the gossipsub topic.
///
/// This function is intended to be spawned via `tokio::spawn(run_heartbeat_loop(30))`.
pub async fn run_heartbeat_loop(interval_secs: u64) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
    loop {
        interval.tick().await;
        // In production, this will:
        // 1. Acquire a lock on the AgentInstance
        // 2. Call agent.heartbeat() to get the payload
        // 3. Serialize and publish via gossipsub topic "wc/heartbeat/1.0"
        // 4. Parse the HeartbeatResponse for lease offers
        tracing::debug!("Heartbeat tick (interval={}s)", interval_secs);
    }
}
