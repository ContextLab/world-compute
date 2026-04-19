//! Integration tests for the mesh LLM inference pipeline (T199-T202).

use worldcompute::agent::mesh_llm::aggregator::{
    aggregate_expert_responses, sample_token_from_entries, ExpertResponse, LogitEntry,
};
use worldcompute::agent::mesh_llm::expert::ExpertNode;
use worldcompute::agent::mesh_llm::router::MeshRouter;
use worldcompute::agent::mesh_llm::safety::{ActionTier, KillSwitch};
use worldcompute::agent::mesh_llm::self_prompt::{generate_self_tasks, ClusterMetrics, TaskDomain};
use worldcompute::agent::mesh_llm::service::{MeshLLMServiceHandler, MIN_EXPERT_COUNT};

/// T199: Register 4 experts → router selects 4 → aggregate mock logits →
/// verify valid token ID returned.
#[test]
fn end_to_end_inference_pipeline() {
    // 1. Create router and register 4 experts.
    let mut router = MeshRouter::new(4);
    for i in 0..4 {
        router.register_expert(ExpertNode::new(
            format!("expert-{i}"),
            "meta-llama/Llama-3.2-1B",
            100.0 + i as f64 * 10.0,
        ));
    }
    assert_eq!(router.expert_count(), 4);

    // 2. Select 4 experts.
    let selected = router.select_experts(4);
    assert_eq!(selected.len(), 4);

    // 3. Build mock expert responses (top-256 logits each).
    let responses: Vec<ExpertResponse> = selected
        .iter()
        .enumerate()
        .map(|(i, expert)| {
            let logits: Vec<LogitEntry> = (0..256)
                .map(|t| LogitEntry {
                    token_id: t,
                    logit: 10.0 - (t as f32) * 0.04 + (i as f32) * 0.01,
                })
                .collect();
            ExpertResponse {
                expert_id: expert.expert_id.clone(),
                top_logits: logits,
                latency_ms: 10 + i as u32,
            }
        })
        .collect();

    // 4. Aggregate and sample.
    let aggregated = aggregate_expert_responses(&responses);
    assert!(!aggregated.is_empty());
    assert!(aggregated.len() <= 256);

    // Deterministic sample (temperature=0 → argmax).
    let token = sample_token_from_entries(&aggregated, 0.0);
    // Token 0 should have the highest logit.
    assert_eq!(token, 0);

    // Non-deterministic sample should return a valid token ID.
    let token_warm = sample_token_from_entries(&aggregated, 1.0);
    assert!(token_warm < 256);
}

/// T200: Trigger kill switch → verify is_halted() returns true, changes_reverted count correct.
#[test]
fn kill_switch_trigger_and_verify() {
    let mut ks = KillSwitch::new();
    assert!(!ks.is_halted());

    let changes =
        vec!["config-update-1".to_string(), "deploy-v2".to_string(), "scheduler-tune".to_string()];
    ks.trigger("governance-admin", &changes);

    assert!(ks.is_halted());
    assert_eq!(ks.triggered_by.as_deref(), Some("governance-admin"));
    assert_eq!(ks.changes_to_revert.len(), 3);
}

/// T200 (continued): Kill switch via service handler halts streams.
#[test]
fn service_handler_halt_reverts_changes() {
    let mut handler = MeshLLMServiceHandler::new(MeshRouter::new(4));

    // Register some experts and add streams.
    for i in 0..5 {
        handler.register_expert(ExpertNode::new(format!("e{i}"), "model", 100.0)).unwrap();
    }
    handler.add_stream();
    handler.add_stream();
    handler.add_stream();

    let result = handler.halt_mesh("emergency-admin");
    assert!(result.halted);
    assert_eq!(result.streams_stopped, 3);
    assert!(handler.kill_switch.is_halted());

    // After halt, registration should be rejected.
    assert!(handler.register_expert(ExpertNode::new("new", "model", 100.0)).is_err());
}

/// T201: Generate self-tasks from metrics → verify at least one task produced,
/// action tier assigned.
#[test]
fn self_task_generation_from_metrics() {
    let metrics = ClusterMetrics {
        cpu_utilization: 0.92,
        memory_utilization: 0.7,
        job_completion_rate: 0.85,
        security_events_24h: 3,
        storage_utilization: 0.91,
    };

    let tasks = generate_self_tasks(&metrics);
    assert!(!tasks.is_empty(), "should generate at least one task for stressed cluster");

    // Verify expected domains present.
    let domains: Vec<TaskDomain> = tasks.iter().map(|t| t.domain).collect();
    assert!(domains.contains(&TaskDomain::SchedulerOptimization));
    assert!(domains.contains(&TaskDomain::SecurityAudit));
    assert!(domains.contains(&TaskDomain::StorageCompaction));
    assert!(domains.contains(&TaskDomain::NetworkTopology));

    // Every task must have a valid priority and action tier.
    for task in &tasks {
        assert!(task.priority >= 1 && task.priority <= 3);
        assert!(
            task.action_tier == ActionTier::ReadOnly || task.action_tier == ActionTier::Suggest
        );
    }
}

/// T198: Degradation warning when expert_count < 280.
#[test]
fn degradation_warning_below_minimum() {
    let mut handler = MeshLLMServiceHandler::new(MeshRouter::new(4));
    // Register fewer than MIN_EXPERT_COUNT experts.
    for i in 0..10 {
        handler.register_expert(ExpertNode::new(format!("e{i}"), "model", 100.0)).unwrap();
    }
    let status = handler.get_router_status();
    assert_eq!(status.expert_count, 10);
    assert!(
        status.health.contains("degraded"),
        "status should warn about degradation, got: {}",
        status.health
    );
    assert!(
        status.health.contains(&MIN_EXPERT_COUNT.to_string()),
        "warning should mention minimum count"
    );
}
