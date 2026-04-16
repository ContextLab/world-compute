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
}
