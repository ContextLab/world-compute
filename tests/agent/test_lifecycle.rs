//! Integration tests for agent lifecycle (T040-T041).

use worldcompute::acceptable_use::AcceptableUseClass;
use worldcompute::agent::config::AgentConfig;
use worldcompute::agent::lifecycle::{AgentInstance, HeartbeatResponse};
use worldcompute::agent::AgentState;

fn test_config() -> AgentConfig {
    let dir = std::env::temp_dir().join(format!("wc-integ-agent-{}", uuid::Uuid::new_v4()));
    AgentConfig { work_dir: dir.clone(), key_path: dir.join("test-key"), ..AgentConfig::default() }
}

#[test]
fn heartbeat_creates_valid_payload() {
    let config = test_config();
    let mut agent = AgentInstance::new(config);
    agent.enroll(vec![AcceptableUseClass::Scientific]).unwrap();

    let payload = agent.heartbeat().unwrap();

    // Verify all fields are populated
    assert!(!payload.node_id.is_empty(), "node_id must be set");
    assert!(!payload.state.is_empty(), "state must be set");
    assert_eq!(payload.active_leases, 0);
    assert_eq!(payload.resource_usage.cpu_percent, 0.0);
    assert_eq!(payload.resource_usage.memory_mb, 0);
    assert_eq!(payload.resource_usage.disk_mb, 0);

    // Verify payload serializes to valid JSON
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("node_id"));
    assert!(json.contains("resource_usage"));
    assert!(json.contains("active_leases"));

    // Verify HeartbeatResponse can be deserialized
    let response_json = r#"{"lease_offers":["lease-1","lease-2"]}"#;
    let response: HeartbeatResponse = serde_json::from_str(response_json).unwrap();
    assert_eq!(response.lease_offers.len(), 2);

    let _ = agent.withdraw();
}

#[test]
fn pause_transitions_state_and_returns_sandbox_list() {
    let config = test_config();
    let mut agent = AgentInstance::new(config);
    agent.enroll(vec![]).unwrap();

    // Add some sandbox IDs to simulate active work
    agent.active_sandbox_ids.push("sandbox-aaa".to_string());
    agent.active_sandbox_ids.push("sandbox-bbb".to_string());

    let result = agent.pause().unwrap();

    assert_eq!(agent.state, AgentState::Paused);
    assert_eq!(result.sandbox_ids.len(), 2);
    assert!(result.sandbox_ids.contains(&"sandbox-aaa".to_string()));
    assert!(result.sandbox_ids.contains(&"sandbox-bbb".to_string()));

    let _ = agent.withdraw();
}

#[test]
fn withdraw_returns_complete_report() {
    let config = test_config();
    std::fs::create_dir_all(&config.work_dir).unwrap();
    let mut agent = AgentInstance::new(config.clone());
    agent.enroll(vec![]).unwrap();

    // Simulate active sandboxes
    agent.active_sandbox_ids.push("sb-1".to_string());
    agent.active_sandbox_ids.push("sb-2".to_string());
    agent.active_sandbox_ids.push("sb-3".to_string());

    let report = agent.withdraw().unwrap();

    assert!(report.keypair_revoked);
    assert!(report.work_dir_wiped);
    assert_eq!(report.processes_terminated, 3);
    assert!(report.network_state_cleared);
    assert!(!config.work_dir.exists(), "Work dir should be removed");
    assert!(agent.donor.is_none());
    assert!(agent.node.is_none());
}

#[test]
fn rapid_pause_resume_cycling() {
    let config = test_config();
    let mut agent = AgentInstance::new(config);
    agent.enroll(vec![]).unwrap();

    // Rapidly cycle pause/resume 10 times — state must remain consistent
    for i in 0..10 {
        let pause_result = agent.pause().unwrap();
        assert_eq!(agent.state, AgentState::Paused, "Cycle {i}: should be Paused");
        assert!(pause_result.sandbox_ids.is_empty());

        agent.resume().unwrap();
        assert_eq!(agent.state, AgentState::Idle, "Cycle {i}: should be Idle after resume");
    }

    // Final state is Idle
    assert_eq!(agent.state, AgentState::Idle);

    let _ = agent.withdraw();
}
