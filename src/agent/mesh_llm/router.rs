//! Router — selects K-of-N experts for each inference request (FR-122).

use rand::seq::SliceRandom;

use super::expert::{ExpertNode, ExpertRegistry};

/// Configuration for the mesh LLM router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Number of experts to select per request.
    pub k_experts: usize,
    /// LLaMA-3 vocabulary size (128K tokens).
    pub tokenizer_vocab_size: u32,
    /// Sampling temperature applied after logit aggregation.
    pub temperature: f64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self { k_experts: 3, tokenizer_vocab_size: 128_000, temperature: 1.0 }
    }
}

/// Result of expert selection: which experts were chosen and their weights.
#[derive(Debug, Clone)]
pub struct ExpertSelection {
    /// IDs of the selected experts.
    pub expert_ids: Vec<String>,
    /// Weights assigned to each expert (sums to 1.0).
    pub weights: Vec<f64>,
}

/// Select `k` experts uniformly at random from `available_experts`.
///
/// Weights are assigned uniformly so they sum to 1.0.
/// Returns an empty selection if `available_experts` is empty or `k == 0`.
pub fn select_experts(available_experts: &[String], k: usize) -> ExpertSelection {
    if available_experts.is_empty() || k == 0 {
        return ExpertSelection { expert_ids: vec![], weights: vec![] };
    }

    let k = k.min(available_experts.len());
    let mut rng = rand::thread_rng();
    let mut pool: Vec<&String> = available_experts.iter().collect();
    pool.shuffle(&mut rng);
    let selected: Vec<String> = pool.into_iter().take(k).cloned().collect();
    let weight = 1.0 / k as f64;
    let weights = vec![weight; k];

    ExpertSelection { expert_ids: selected, weights }
}

// ---------------------------------------------------------------------------
// MeshRouter (T189) — higher-level router that owns an ExpertRegistry
// ---------------------------------------------------------------------------

/// High-level mesh router that selects K healthiest, lowest-latency experts
/// per token from a local expert registry.
#[derive(Debug)]
pub struct MeshRouter {
    pub registry: ExpertRegistry,
    /// Experts per token (default 4).
    pub k: usize,
    /// Tokenizer family — must be "llama3".
    pub tokenizer_name: String,
}

impl MeshRouter {
    /// Create a new router that selects `k` experts per token.
    pub fn new(k: usize) -> Self {
        Self { registry: ExpertRegistry::new(), k, tokenizer_name: "llama3".to_string() }
    }

    /// Register an expert node with the router.
    pub fn register_expert(&mut self, expert: ExpertNode) {
        // Silently ignore duplicates at router level (registry returns error).
        let _ = self.registry.register_expert(expert);
    }

    /// Select the K healthiest experts. Prefers `Online` experts sorted by
    /// highest throughput (tokens/sec), then takes the top `k`.
    pub fn select_experts(&self, k: usize) -> Vec<&ExpertNode> {
        let mut healthy = self.registry.get_healthy();
        // Sort by throughput descending (proxy for lowest latency / best health).
        healthy.sort_by(|a, b| {
            b.capacity_tokens_per_sec
                .partial_cmp(&a.capacity_tokens_per_sec)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        healthy.truncate(k);
        healthy
    }

    /// Number of registered experts (any status).
    pub fn expert_count(&self) -> usize {
        self.registry.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn experts(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("expert-{i}")).collect()
    }

    #[test]
    fn selection_returns_k_experts() {
        let pool = experts(10);
        let sel = select_experts(&pool, 3);
        assert_eq!(sel.expert_ids.len(), 3);
        assert_eq!(sel.weights.len(), 3);
    }

    #[test]
    fn weights_sum_to_one() {
        let pool = experts(8);
        let sel = select_experts(&pool, 4);
        let sum: f64 = sel.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "weights sum {sum} != 1.0");
    }

    #[test]
    fn k_capped_at_available() {
        let pool = experts(2);
        let sel = select_experts(&pool, 10);
        assert_eq!(sel.expert_ids.len(), 2);
        let sum: f64 = sel.weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn empty_pool_returns_empty() {
        let sel = select_experts(&[], 5);
        assert!(sel.expert_ids.is_empty());
        assert!(sel.weights.is_empty());
    }

    #[test]
    fn k_zero_returns_empty() {
        let pool = experts(5);
        let sel = select_experts(&pool, 0);
        assert!(sel.expert_ids.is_empty());
    }

    #[test]
    fn selected_ids_are_from_pool() {
        let pool = experts(10);
        let sel = select_experts(&pool, 5);
        for id in &sel.expert_ids {
            assert!(pool.contains(id), "{id} not in pool");
        }
    }

    #[test]
    fn default_router_config() {
        let cfg = RouterConfig::default();
        assert_eq!(cfg.tokenizer_vocab_size, 128_000);
        assert_eq!(cfg.k_experts, 3);
    }

    #[test]
    fn mesh_router_new() {
        let router = MeshRouter::new(4);
        assert_eq!(router.k, 4);
        assert_eq!(router.tokenizer_name, "llama3");
        assert_eq!(router.expert_count(), 0);
    }

    #[test]
    fn mesh_router_register_and_count() {
        let mut router = MeshRouter::new(4);
        router.register_expert(ExpertNode::new("e1", "model-a", 100.0));
        router.register_expert(ExpertNode::new("e2", "model-b", 200.0));
        assert_eq!(router.expert_count(), 2);
    }

    #[test]
    fn mesh_router_select_by_throughput() {
        let mut router = MeshRouter::new(2);
        router.register_expert(ExpertNode::new("slow", "m", 50.0));
        router.register_expert(ExpertNode::new("fast", "m", 300.0));
        router.register_expert(ExpertNode::new("mid", "m", 150.0));
        let selected = router.select_experts(2);
        assert_eq!(selected.len(), 2);
        // Fastest should be first.
        assert_eq!(selected[0].expert_id, "fast");
        assert_eq!(selected[1].expert_id, "mid");
    }

    #[test]
    fn mesh_router_select_caps_at_available() {
        let mut router = MeshRouter::new(4);
        router.register_expert(ExpertNode::new("only", "m", 100.0));
        let selected = router.select_experts(10);
        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn mesh_router_select_skips_offline() {
        use crate::agent::mesh_llm::expert::ExpertStatus;
        let mut router = MeshRouter::new(4);
        let mut off = ExpertNode::new("off", "m", 500.0);
        off.status = ExpertStatus::Offline;
        router.register_expert(off);
        router.register_expert(ExpertNode::new("on", "m", 100.0));
        let selected = router.select_experts(4);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].expert_id, "on");
    }
}
