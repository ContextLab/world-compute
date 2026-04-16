use serde_json::{json, Value};

pub fn get_donor_status() -> Value {
    json!({
        "status": "stub",
        "donor_id": null,
        "compute_contributed_hours": 0,
        "tokens_earned": 0,
        "agent_running": false
    })
}

pub fn get_job_status() -> Value {
    json!({
        "status": "stub",
        "job_id": null,
        "state": "unknown",
        "progress": 0,
        "result": null
    })
}

pub fn get_cluster_status() -> Value {
    json!({
        "status": "stub",
        "nodes_online": 0,
        "jobs_queued": 0,
        "jobs_running": 0,
        "total_compute_hours": 0
    })
}

pub fn get_mesh_status() -> Value {
    json!({
        "status": "stub",
        "mesh_nodes": 0,
        "active_inference_sessions": 0,
        "model_shards_hosted": 0
    })
}

pub fn submit_job() -> Value {
    json!({
        "status": "stub",
        "job_id": null,
        "message": "job submission not yet implemented"
    })
}

pub fn pause_agent() -> Value {
    json!({
        "status": "stub",
        "message": "pause_agent not yet implemented"
    })
}

pub fn resume_agent() -> Value {
    json!({
        "status": "stub",
        "message": "resume_agent not yet implemented"
    })
}
