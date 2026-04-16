//! Agent subsetting — carve off independent parallel agent groups (FR-124).

use crate::agent::mesh_llm::expert::ExpertRegistry;
use crate::agent::mesh_llm::self_prompt::SelfPromptTask;

/// A subset of experts assigned to work on a specific task in parallel.
#[derive(Debug, Clone)]
pub struct AgentSubset {
    pub subset_id: String,
    pub expert_ids: Vec<String>,
    pub task: SelfPromptTask,
}

/// Partition online experts from `registry` into `num_subsets` groups using
/// round-robin assignment. Experts with no task are not assigned.
///
/// Returns an empty `Vec` when `num_subsets == 0` or the registry has no
/// online experts.
pub fn partition_experts(registry: &ExpertRegistry, num_subsets: usize) -> Vec<AgentSubset> {
    if num_subsets == 0 {
        return vec![];
    }

    let online = registry.list_online_experts();
    if online.is_empty() {
        return vec![];
    }

    // Cycle through task variants deterministically.
    const TASKS: &[SelfPromptTask] = &[
        SelfPromptTask::SchedulerOptimization,
        SelfPromptTask::SecurityLogAnalysis,
        SelfPromptTask::TestGeneration,
        SelfPromptTask::ConfigTuning,
        SelfPromptTask::GovernanceProposalDraft,
    ];

    let mut subsets: Vec<AgentSubset> = (0..num_subsets)
        .map(|i| AgentSubset {
            subset_id: format!("subset-{i}"),
            expert_ids: vec![],
            task: TASKS[i % TASKS.len()],
        })
        .collect();

    // Round-robin assignment.
    for (idx, expert_id) in online.into_iter().enumerate() {
        subsets[idx % num_subsets].expert_ids.push(expert_id);
    }

    subsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::mesh_llm::expert::{ExpertNode, ExpertRegistry, ExpertStatus};

    fn filled_registry(n: usize) -> ExpertRegistry {
        let mut reg = ExpertRegistry::new();
        for i in 0..n {
            reg.register_expert(ExpertNode::new(format!("e{i}"), "meta-llama/Llama-3.2-1B", 100.0))
                .unwrap();
        }
        reg
    }

    #[test]
    fn partition_distributes_evenly() {
        let reg = filled_registry(6);
        let subsets = partition_experts(&reg, 3);
        assert_eq!(subsets.len(), 3);
        // Each subset should have exactly 2 experts.
        for s in &subsets {
            assert_eq!(s.expert_ids.len(), 2, "subset {} has wrong count", s.subset_id);
        }
        // Total expert count equals 6.
        let total: usize = subsets.iter().map(|s| s.expert_ids.len()).sum();
        assert_eq!(total, 6);
    }

    #[test]
    fn empty_registry_returns_empty() {
        let reg = ExpertRegistry::new();
        let subsets = partition_experts(&reg, 4);
        assert!(subsets.is_empty());
    }

    #[test]
    fn zero_subsets_returns_empty() {
        let reg = filled_registry(5);
        let subsets = partition_experts(&reg, 0);
        assert!(subsets.is_empty());
    }

    #[test]
    fn offline_experts_excluded() {
        let mut reg = ExpertRegistry::new();
        let mut offline = ExpertNode::new("offline", "model", 50.0);
        offline.status = ExpertStatus::Offline;
        reg.register_expert(offline).unwrap();
        reg.register_expert(ExpertNode::new("online", "model", 50.0)).unwrap();

        let subsets = partition_experts(&reg, 2);
        let all_ids: Vec<&String> = subsets.iter().flat_map(|s| &s.expert_ids).collect();
        assert!(!all_ids.iter().any(|id| id.as_str() == "offline"));
        assert!(all_ids.iter().any(|id| id.as_str() == "online"));
    }

    #[test]
    fn subset_ids_are_unique() {
        let reg = filled_registry(4);
        let subsets = partition_experts(&reg, 4);
        let mut ids: Vec<&str> = subsets.iter().map(|s| s.subset_id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }
}
