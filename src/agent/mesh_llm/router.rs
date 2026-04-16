//! Router — selects K-of-N experts for each inference request (FR-122).

use rand::seq::SliceRandom;

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
}
