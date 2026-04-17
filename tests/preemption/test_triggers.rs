//! Integration tests for preemption trigger types (T107).

use worldcompute::preemption::supervisor::{
    PreemptionEvent, PreemptionHandlerResult, PreemptionResult,
};

#[test]
fn event_creation_keyboard() {
    let event = PreemptionEvent::KeyboardActivity;
    assert_eq!(event, PreemptionEvent::KeyboardActivity);
}

#[test]
fn event_creation_all_variants() {
    let events = [
        PreemptionEvent::KeyboardActivity,
        PreemptionEvent::MouseActivity,
        PreemptionEvent::ThermalThreshold,
        PreemptionEvent::BatteryDisconnect,
        PreemptionEvent::MemoryPressure,
    ];
    // All variants should be distinct
    for i in 0..events.len() {
        for j in (i + 1)..events.len() {
            assert_ne!(events[i], events[j]);
        }
    }
}

#[test]
fn preemption_result_within_budget() {
    let result = PreemptionResult {
        frozen_count: 3,
        freeze_latency_us: 5_000, // 5ms, well within 10ms budget
        errors: Vec::new(),
    };
    assert!(result.within_budget());
    assert_eq!(result.frozen_count, 3);
    assert!(result.errors.is_empty());
}

#[test]
fn preemption_result_over_budget() {
    let result = PreemptionResult {
        frozen_count: 1,
        freeze_latency_us: 15_000, // 15ms, over 10ms budget
        errors: vec!["slow sandbox".into()],
    };
    assert!(!result.within_budget());
}

#[test]
fn handler_result_fields() {
    let result = PreemptionHandlerResult {
        event: PreemptionEvent::MemoryPressure,
        sandbox_pids_stopped: 2,
        latency_ns: 500_000,
        checkpoint_attempted: true,
        checkpoint_succeeded: true,
    };
    assert_eq!(result.event, PreemptionEvent::MemoryPressure);
    assert_eq!(result.sandbox_pids_stopped, 2);
    assert!(result.checkpoint_attempted);
    assert!(result.checkpoint_succeeded);
}
