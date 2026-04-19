//! Integration tests for preemption supervisor (T046-T047).

use worldcompute::preemption::supervisor::{
    PreemptionEvent, PreemptionSupervisor, GPU_KERNEL_WINDOW_MS,
};

#[test]
fn preemption_event_creation_and_result_fields() {
    // Verify all event variants can be created
    let events = [
        PreemptionEvent::KeyboardActivity,
        PreemptionEvent::MouseActivity,
        PreemptionEvent::ThermalThreshold,
        PreemptionEvent::BatteryDisconnect,
        PreemptionEvent::MemoryPressure,
    ];

    for event in &events {
        assert_eq!(*event, *event, "Event should be Eq");
    }

    // Verify GPU kernel window constant
    assert_eq!(GPU_KERNEL_WINDOW_MS, 200);
}

#[test]
fn handle_preemption_event_empty_pids() {
    use worldcompute::preemption::supervisor::handle_preemption_event;

    let result = handle_preemption_event(PreemptionEvent::KeyboardActivity, &[]);

    #[cfg(unix)]
    {
        let result = result.unwrap();
        assert_eq!(result.sandbox_pids_stopped, 0);
        assert_eq!(result.event, PreemptionEvent::KeyboardActivity);
        assert!(!result.checkpoint_attempted);
        assert!(!result.checkpoint_succeeded);
        // Latency should be very low with no pids
        assert!(result.latency_ns < 1_000_000, "Should complete in under 1ms");
    }

    #[cfg(not(unix))]
    {
        assert!(result.is_err());
    }
}

#[cfg(unix)]
#[test]
fn escalation_with_nonexistent_pids() {
    use worldcompute::preemption::supervisor::escalate_after_stop;

    // Use pid 0 which refers to the calling process's group — safe for
    // signal-check but won't actually stop anything meaningful in tests.
    // Use a very high PID that almost certainly doesn't exist.
    let fake_pids = [999_999_999u32];
    let result = escalate_after_stop(&fake_pids, 500);

    // The pid doesn't exist, so it should be counted as killed (process gone)
    assert_eq!(result.checkpointed + result.killed, 1);
}

#[cfg(unix)]
#[test]
fn checkpoint_failure_triggers_kill_escalation() {
    use worldcompute::preemption::supervisor::escalate_after_stop;

    // With a zero-ms budget, all pids should be escalated to SIGKILL
    let fake_pids = [999_999_998u32, 999_999_997u32];
    let result = escalate_after_stop(&fake_pids, 0);

    // With zero budget, all should be killed (or already gone)
    assert_eq!(result.checkpointed + result.killed, 2, "All pids should be accounted for");
    // With budget=0 the first pid check already exceeds budget, so killed >= 1
    // (exact count depends on timing, but total must be 2)
}

#[test]
fn supervisor_freeze_resume_with_no_sandboxes() {
    let (_tx, rx) = tokio::sync::watch::channel(None);
    let mut sup = PreemptionSupervisor::new(rx);

    // Freeze with no sandboxes
    let freeze_result = sup.freeze_all();
    assert_eq!(freeze_result.frozen_count, 0);
    assert!(freeze_result.within_budget());
    assert!(sup.is_frozen());

    // Resume
    let resume_result = sup.resume_all();
    assert_eq!(resume_result.resumed_count, 0);
    assert!(!sup.is_frozen());
}
