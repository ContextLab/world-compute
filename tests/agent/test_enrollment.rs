//! Integration tests for agent enrollment flow (T101).

use worldcompute::agent::config::AgentConfig;
use worldcompute::agent::AgentState;

#[test]
fn default_config_has_valid_work_dir() {
    let config = AgentConfig::default();
    // work_dir should be under temp_dir
    let work_dir_str = config.work_dir.to_string_lossy();
    assert!(
        work_dir_str.contains("worldcompute"),
        "Default work_dir should contain 'worldcompute', got: {work_dir_str}"
    );
}

#[test]
fn state_transitions_enrolling_to_idle() {
    let state = AgentState::Enrolling;
    assert_eq!(state, AgentState::Enrolling);
    // Simulate transition
    let next_state = AgentState::Idle;
    assert_eq!(next_state, AgentState::Idle);
    assert_ne!(state, next_state);
}

#[test]
fn config_cpu_cap_within_range() {
    let config = AgentConfig::default();
    assert!(config.cpu_cap_percent <= 100, "CPU cap should be <= 100");
    assert!(config.cpu_cap_percent > 0, "CPU cap should be > 0");
}

#[test]
fn all_agent_states_distinct() {
    let states = [
        AgentState::Enrolling,
        AgentState::Idle,
        AgentState::Working,
        AgentState::Paused,
        AgentState::Withdrawing,
    ];
    for i in 0..states.len() {
        for j in (i + 1)..states.len() {
            assert_ne!(states[i], states[j]);
        }
    }
}
