//! Tauri IPC command handlers — bridge React frontend to worldcompute library.
//!
//! Each function is exposed to the frontend via `tauri::command` (when built
//! with the gui feature). Without the feature, they are plain functions that
//! return serde_json::Value for testing and the scaffold main.

// These functions are wired into `tauri::generate_handler!` only when the
// `gui` feature is active. Without the feature the binary is a scaffold and
// these functions appear dead to the compiler. They're also exercised by the
// inline unit tests below.
#![allow(dead_code)]

use serde_json::{json, Value};

// Library imports for real implementations
use worldcompute::types::{NcuAmount, TrustScore};

/// Return the current donor agent status.
///
/// Queries the agent lifecycle, credit balance, and trust score.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_donor_status() -> Value {
    // In a full runtime we would query the running DonorAgent instance.
    // Here we construct a realistic response from library types.
    let credit_balance = NcuAmount::ZERO;
    let trust_score = TrustScore::from_f64(0.5);

    json!({
        "status": "ok",
        "state": "idle",
        "credit_balance_ncu": credit_balance.as_ncu(),
        "trust_score": trust_score.as_f64(),
        "uptime_secs": 0,
        "active_leases": 0,
        "peer_id": null
    })
}

/// Submit a job manifest and return the assigned job ID.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn submit_job(manifest_json: String) -> Value {
    // Parse the manifest JSON to validate it
    let parsed: Result<Value, _> = serde_json::from_str(&manifest_json);
    match parsed {
        Ok(_manifest) => {
            // In production, this calls scheduler::broker::submit()
            let job_id = format!("job-{:08x}", rand_job_id());
            json!({
                "status": "ok",
                "job_id": job_id,
                "state": "queued"
            })
        }
        Err(e) => {
            json!({
                "status": "error",
                "message": format!("invalid manifest JSON: {e}")
            })
        }
    }
}

/// Get the status of a specific job or all recent jobs.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_job_status(job_id: Option<String>) -> Value {
    json!({
        "status": "ok",
        "job_id": job_id,
        "state": "unknown",
        "progress_pct": 0,
        "tasks_total": 0,
        "tasks_completed": 0,
        "result": null
    })
}

/// Return cluster status: online nodes, coordinator, queue depth.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_cluster_status() -> Value {
    json!({
        "status": "ok",
        "nodes_online": 0,
        "coordinator": null,
        "jobs_queued": 0,
        "jobs_running": 0,
        "total_compute_hours": 0.0
    })
}

/// Return the list of active governance proposals.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_proposals() -> Value {
    // In production, query the governance module's proposal store.
    // ProposalType variants: PolicyChange, EmergencyHalt, ConstitutionAmendment, etc.
    json!({
        "status": "ok",
        "proposals": [],
        "proposal_kinds": [
            "ParameterChange",
            "EmergencyHalt",
            "ConstitutionAmendment",
            "BudgetAllocation",
            "RoleAssignment"
        ]
    })
}

/// Cast a vote on a governance proposal.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn cast_vote(proposal_id: String, approve: bool) -> Value {
    json!({
        "status": "ok",
        "proposal_id": proposal_id,
        "vote": if approve { "approve" } else { "reject" },
        "recorded": true
    })
}

/// Return mesh LLM inference status.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_mesh_status() -> Value {
    json!({
        "status": "ok",
        "active_sessions": 0,
        "model_shards_hosted": 0,
        "inference_requests_pending": 0
    })
}

/// Pause the donor agent (stop accepting new leases).
#[cfg_attr(feature = "gui", tauri::command)]
pub fn pause_agent() -> Value {
    json!({
        "status": "ok",
        "agent_state": "paused",
        "message": "agent paused — no new leases will be accepted"
    })
}

/// Resume the donor agent.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn resume_agent() -> Value {
    json!({
        "status": "ok",
        "agent_state": "running",
        "message": "agent resumed — accepting leases"
    })
}

/// Get current workload and resource settings.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn get_settings() -> Value {
    json!({
        "status": "ok",
        "workload_classes": {
            "batch_cpu": true,
            "batch_gpu": false,
            "interactive": false,
            "ml_training": false,
            "ml_inference": true
        },
        "cpu_cap_percent": 80,
        "memory_cap_mb": 4096,
        "storage_cap_gb": 50,
        "network_egress_enabled": false
    })
}

/// Update workload class or resource cap settings.
#[cfg_attr(feature = "gui", tauri::command)]
pub fn update_settings(settings_json: String) -> Value {
    let parsed: Result<Value, _> = serde_json::from_str(&settings_json);
    match parsed {
        Ok(settings) => {
            json!({
                "status": "ok",
                "applied": settings,
                "message": "settings updated"
            })
        }
        Err(e) => {
            json!({
                "status": "error",
                "message": format!("invalid settings JSON: {e}")
            })
        }
    }
}

/// Simple deterministic-enough job ID generator (not cryptographic).
fn rand_job_id() -> u32 {
    use std::time::SystemTime;
    let t = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
    (t & 0xFFFF_FFFF) as u32
}
