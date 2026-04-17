//! Self-prompting loop — mesh generates improvement tasks for itself (FR-123).

use crate::types::Timestamp;

/// Categories of tasks the mesh can generate for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfPromptTask {
    SchedulerOptimization,
    SecurityLogAnalysis,
    TestGeneration,
    ConfigTuning,
    GovernanceProposalDraft,
}

/// Output from a completed self-prompting cycle.
#[derive(Debug, Clone)]
pub struct SelfPromptResult {
    pub task: SelfPromptTask,
    pub output_text: String,
    /// Safety tier under which the output action falls.
    pub action_tier: String,
    pub timestamp: Timestamp,
}

/// Generate the prompt text that will be fed back into the mesh for a given task.
pub fn generate_task_prompt(task: SelfPromptTask) -> String {
    match task {
        SelfPromptTask::SchedulerOptimization => {
            "Analyze the current task scheduler's queue depth, latency percentiles, and \
             resource utilization over the last 24 hours. Identify at least three concrete \
             configuration changes or algorithmic improvements that would reduce p99 latency \
             by at least 10%. For each suggestion, provide the specific parameter names, \
             proposed values, and an estimate of expected impact."
                .to_string()
        }
        SelfPromptTask::SecurityLogAnalysis => {
            "Review the last 1000 security log entries from the cluster audit trail. \
             Identify anomalous patterns, failed authentication attempts, unusual data-access \
             paths, or privilege-escalation indicators. Summarize findings by severity \
             (critical/high/medium/low) and propose mitigations for any critical or high \
             severity items."
                .to_string()
        }
        SelfPromptTask::TestGeneration => {
            "Examine the current test coverage report and identify the five modules with the \
             lowest branch coverage. For each module, generate a suite of unit tests that \
             exercises the uncovered branches, including edge cases and error paths. \
             Output valid Rust test code inside ```rust code blocks."
                .to_string()
        }
        SelfPromptTask::ConfigTuning => {
            "Inspect the current runtime configuration of the World Compute cluster. \
             Propose tuning adjustments for network timeouts, consensus quorum sizes, \
             erasure-coding shard ratios, and gossip fanout to optimize for the current \
             node-count and workload mix. Provide a diff of proposed changes against \
             the current config."
                .to_string()
        }
        SelfPromptTask::GovernanceProposalDraft => {
            "Draft a governance proposal for the World Compute community that addresses \
             an identified operational need or policy gap. The proposal must include: \
             (1) a title, (2) a problem statement, (3) proposed changes to policy or \
             parameters, (4) expected benefits and risks, (5) a rollback plan, and \
             (6) a list of stakeholders who should review the proposal before it is \
             opened for voting."
                .to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Self-task generation from cluster metrics (T194)
// ---------------------------------------------------------------------------

use super::safety::ActionTier;

/// Domain categories for self-generated tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDomain {
    SchedulerOptimization,
    SecurityAudit,
    StorageCompaction,
    NetworkTopology,
}

/// A self-generated improvement task.
#[derive(Debug, Clone)]
pub struct SelfTask {
    pub description: String,
    pub domain: TaskDomain,
    pub priority: u8,
    pub action_tier: ActionTier,
}

/// Cluster-wide metrics used to decide which self-tasks to generate.
#[derive(Debug, Clone)]
pub struct ClusterMetrics {
    /// CPU utilization as a fraction in [0.0, 1.0].
    pub cpu_utilization: f64,
    /// Memory utilization as a fraction in [0.0, 1.0].
    pub memory_utilization: f64,
    /// Fraction of jobs completed successfully in [0.0, 1.0].
    pub job_completion_rate: f64,
    /// Number of security events in the last 24 hours.
    pub security_events_24h: u32,
    /// Storage utilization as a fraction in [0.0, 1.0].
    pub storage_utilization: f64,
}

/// Generate improvement tasks based on cluster metrics.
///
/// Rules:
/// - High CPU utilization (>0.8) → scheduler optimization task
/// - Recent security events (>0) → security audit task
/// - Storage near capacity (>0.85) → storage compaction task
/// - Low job completion rate (<0.9) → network topology review
pub fn generate_self_tasks(metrics: &ClusterMetrics) -> Vec<SelfTask> {
    let mut tasks = Vec::new();

    if metrics.cpu_utilization > 0.8 {
        tasks.push(SelfTask {
            description: format!(
                "CPU utilization at {:.0}% — analyze scheduler queue depth and rebalance",
                metrics.cpu_utilization * 100.0
            ),
            domain: TaskDomain::SchedulerOptimization,
            priority: if metrics.cpu_utilization > 0.95 { 1 } else { 2 },
            action_tier: ActionTier::Suggest,
        });
    }

    if metrics.security_events_24h > 0 {
        tasks.push(SelfTask {
            description: format!(
                "{} security events in last 24h — review audit trail and recommend mitigations",
                metrics.security_events_24h
            ),
            domain: TaskDomain::SecurityAudit,
            priority: if metrics.security_events_24h > 10 { 1 } else { 3 },
            action_tier: ActionTier::ReadOnly,
        });
    }

    if metrics.storage_utilization > 0.85 {
        tasks.push(SelfTask {
            description: format!(
                "Storage at {:.0}% — identify stale data and suggest compaction",
                metrics.storage_utilization * 100.0
            ),
            domain: TaskDomain::StorageCompaction,
            priority: if metrics.storage_utilization > 0.95 { 1 } else { 3 },
            action_tier: ActionTier::Suggest,
        });
    }

    if metrics.job_completion_rate < 0.9 {
        tasks.push(SelfTask {
            description: format!(
                "Job completion rate at {:.0}% — analyze network topology for bottlenecks",
                metrics.job_completion_rate * 100.0
            ),
            domain: TaskDomain::NetworkTopology,
            priority: 2,
            action_tier: ActionTier::ReadOnly,
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_TASKS: &[SelfPromptTask] = &[
        SelfPromptTask::SchedulerOptimization,
        SelfPromptTask::SecurityLogAnalysis,
        SelfPromptTask::TestGeneration,
        SelfPromptTask::ConfigTuning,
        SelfPromptTask::GovernanceProposalDraft,
    ];

    #[test]
    fn each_task_generates_non_empty_prompt() {
        for &task in ALL_TASKS {
            let prompt = generate_task_prompt(task);
            assert!(!prompt.is_empty(), "prompt for {task:?} must not be empty");
        }
    }

    #[test]
    fn prompts_are_distinct() {
        let prompts: Vec<String> = ALL_TASKS.iter().map(|&t| generate_task_prompt(t)).collect();
        for i in 0..prompts.len() {
            for j in (i + 1)..prompts.len() {
                assert_ne!(
                    prompts[i], prompts[j],
                    "task prompts at index {i} and {j} are identical"
                );
            }
        }
    }

    #[test]
    fn generate_self_tasks_high_cpu() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.95,
            memory_utilization: 0.5,
            job_completion_rate: 0.95,
            security_events_24h: 0,
            storage_utilization: 0.5,
        };
        let tasks = generate_self_tasks(&metrics);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].domain, TaskDomain::SchedulerOptimization);
    }

    #[test]
    fn generate_self_tasks_security_events() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.5,
            memory_utilization: 0.5,
            job_completion_rate: 0.95,
            security_events_24h: 5,
            storage_utilization: 0.5,
        };
        let tasks = generate_self_tasks(&metrics);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].domain, TaskDomain::SecurityAudit);
    }

    #[test]
    fn generate_self_tasks_storage_full() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.5,
            memory_utilization: 0.5,
            job_completion_rate: 0.95,
            security_events_24h: 0,
            storage_utilization: 0.92,
        };
        let tasks = generate_self_tasks(&metrics);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].domain, TaskDomain::StorageCompaction);
    }

    #[test]
    fn generate_self_tasks_low_completion() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.5,
            memory_utilization: 0.5,
            job_completion_rate: 0.7,
            security_events_24h: 0,
            storage_utilization: 0.5,
        };
        let tasks = generate_self_tasks(&metrics);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].domain, TaskDomain::NetworkTopology);
    }

    #[test]
    fn generate_self_tasks_multiple() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.99,
            memory_utilization: 0.9,
            job_completion_rate: 0.5,
            security_events_24h: 20,
            storage_utilization: 0.98,
        };
        let tasks = generate_self_tasks(&metrics);
        assert_eq!(tasks.len(), 4);
        // All tasks should have an action tier assigned.
        for t in &tasks {
            assert!(t.priority >= 1 && t.priority <= 3);
        }
    }

    #[test]
    fn generate_self_tasks_healthy_cluster() {
        let metrics = ClusterMetrics {
            cpu_utilization: 0.3,
            memory_utilization: 0.4,
            job_completion_rate: 0.99,
            security_events_24h: 0,
            storage_utilization: 0.2,
        };
        let tasks = generate_self_tasks(&metrics);
        assert!(tasks.is_empty());
    }
}
