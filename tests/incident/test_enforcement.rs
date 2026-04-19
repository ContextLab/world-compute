//! T072: Incident containment enforcement tests.

use worldcompute::incident::containment::{
    execute_block_submitter, execute_drain_pool, execute_freeze_host, execute_quarantine_class,
    execute_revoke_artifact, ContainmentState,
};

#[test]
fn freeze_empty_pid_list_succeeds_with_zero() {
    let count = execute_freeze_host(&[]).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn quarantine_adds_class_to_rejection_set() {
    let state = ContainmentState::new();
    assert!(!state.is_class_quarantined("crypto-mining"));

    execute_quarantine_class(&state, "crypto-mining");

    assert!(state.is_class_quarantined("crypto-mining"));
    assert!(!state.is_class_quarantined("ml-training"));
}

#[test]
fn quarantine_multiple_classes() {
    let state = ContainmentState::new();
    execute_quarantine_class(&state, "class-a");
    execute_quarantine_class(&state, "class-b");

    assert!(state.is_class_quarantined("class-a"));
    assert!(state.is_class_quarantined("class-b"));
    assert!(!state.is_class_quarantined("class-c"));
}

#[test]
fn block_submitter_adds_to_ban_set() {
    let state = ContainmentState::new();
    assert!(!state.is_submitter_blocked("malicious-user"));

    let cancelled = execute_block_submitter(&state, "malicious-user", 7);

    assert_eq!(cancelled, 7);
    assert!(state.is_submitter_blocked("malicious-user"));
    assert!(!state.is_submitter_blocked("legitimate-user"));
}

#[test]
fn revoke_artifact_removes_from_approved_set() {
    let state = ContainmentState::new();
    assert!(!state.is_artifact_revoked("bafyabc123"));

    let affected = execute_revoke_artifact(&state, "bafyabc123", 4);

    assert_eq!(affected, 4);
    assert!(state.is_artifact_revoked("bafyabc123"));
    assert!(!state.is_artifact_revoked("bafydef456"));
}

#[test]
fn drain_pool_marks_as_draining() {
    let state = ContainmentState::new();
    assert!(!state.is_pool_draining("pool-us-west-2"));

    let migrated = execute_drain_pool(&state, "pool-us-west-2", 12);

    assert_eq!(migrated, 12);
    assert!(state.is_pool_draining("pool-us-west-2"));
    assert!(!state.is_pool_draining("pool-eu-west-1"));
}

#[test]
fn containment_state_starts_empty() {
    let state = ContainmentState::default();
    assert!(!state.is_class_quarantined("anything"));
    assert!(!state.is_submitter_blocked("anything"));
    assert!(!state.is_artifact_revoked("anything"));
    assert!(!state.is_pool_draining("anything"));
    assert!(!state.is_host_frozen("anything"));
}

#[test]
fn frozen_host_tracked_in_state() {
    let state = ContainmentState::new();
    state.frozen_hosts.write().unwrap().insert("host-abc".to_string());
    assert!(state.is_host_frozen("host-abc"));
    assert!(!state.is_host_frozen("host-xyz"));
}
