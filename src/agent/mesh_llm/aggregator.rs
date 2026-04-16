//! Logit aggregation — merges sparse top-256 expert outputs (FR-122).

/// Sparse top-256 logit vector returned by a single expert.
#[derive(Debug, Clone)]
pub struct SparseLogits {
    /// Token IDs (up to 256 entries).
    pub token_ids: Vec<u32>,
    /// Log-probabilities corresponding to each token ID.
    pub log_probs: Vec<f64>,
}

impl SparseLogits {
    /// Maximum number of entries kept per expert output.
    pub const TOP_K: usize = 256;
}

/// Weighted-average aggregation of multiple expert sparse logit vectors.
///
/// For each token that appears in any expert output, the log-probability is
/// averaged across experts using the supplied weights. The result is
/// renormalized and truncated to [`SparseLogits::TOP_K`] entries sorted by
/// descending log-probability.
pub fn aggregate_logits(expert_outputs: Vec<(SparseLogits, f64)>) -> SparseLogits {
    use std::collections::HashMap;

    if expert_outputs.is_empty() {
        return SparseLogits { token_ids: vec![], log_probs: vec![] };
    }

    // Accumulate weighted log-probs for each token.
    let mut acc: HashMap<u32, f64> = HashMap::new();
    let mut weight_sum: HashMap<u32, f64> = HashMap::new();

    for (logits, weight) in &expert_outputs {
        for (tid, lp) in logits.token_ids.iter().zip(logits.log_probs.iter()) {
            *acc.entry(*tid).or_insert(0.0) += weight * lp;
            *weight_sum.entry(*tid).or_insert(0.0) += weight;
        }
    }

    // Normalize by actual total weight seen for each token.
    let mut merged: Vec<(u32, f64)> = acc
        .into_iter()
        .map(|(tid, weighted_sum)| {
            let w = weight_sum[&tid];
            (tid, if w > 0.0 { weighted_sum / w } else { weighted_sum })
        })
        .collect();

    // Sort descending by log-prob, keep top-256.
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(SparseLogits::TOP_K);

    SparseLogits {
        token_ids: merged.iter().map(|(id, _)| *id).collect(),
        log_probs: merged.iter().map(|(_, lp)| *lp).collect(),
    }
}

/// Temperature-scaled softmax sampling from an aggregated logit distribution.
///
/// When `temperature == 0.0` this returns the argmax token deterministically.
/// Panics if `aggregated` is empty.
pub fn sample_token(aggregated: &SparseLogits, temperature: f64) -> u32 {
    assert!(!aggregated.token_ids.is_empty(), "cannot sample from empty logit distribution");

    if temperature == 0.0 {
        // Argmax: highest log-prob (entries are already sorted descending).
        return aggregated.token_ids[0];
    }

    // Scale log-probs by 1/temperature then softmax.
    let scaled: Vec<f64> = aggregated.log_probs.iter().map(|lp| lp / temperature).collect();

    // Numerically stable softmax.
    let max_val = scaled.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = scaled.iter().map(|v| (v - max_val).exp()).collect();
    let sum: f64 = exps.iter().sum();
    let probs: Vec<f64> = exps.iter().map(|e| e / sum).collect();

    // Categorical sampling.
    let mut rng_val: f64 = rand::random::<f64>();
    for (prob, &tid) in probs.iter().zip(aggregated.token_ids.iter()) {
        rng_val -= prob;
        if rng_val <= 0.0 {
            return tid;
        }
    }
    // Fallback due to floating-point rounding.
    *aggregated.token_ids.last().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logits(ids: &[u32], lps: &[f64]) -> SparseLogits {
        SparseLogits { token_ids: ids.to_vec(), log_probs: lps.to_vec() }
    }

    #[test]
    fn single_expert_passthrough() {
        let input = logits(&[1, 2, 3], &[-1.0, -2.0, -3.0]);
        let out = aggregate_logits(vec![(input, 1.0)]);
        // All three tokens present, ordering preserved (descending by log-prob).
        assert_eq!(out.token_ids[0], 1);
        assert!((out.log_probs[0] - -1.0).abs() < 1e-10);
    }

    #[test]
    fn two_equal_experts_average() {
        let a = logits(&[10, 20], &[-1.0, -3.0]);
        let b = logits(&[10, 20], &[-3.0, -1.0]);
        let out = aggregate_logits(vec![(a, 0.5), (b, 0.5)]);

        // Token 10: avg of -1.0 and -3.0 = -2.0
        // Token 20: avg of -3.0 and -1.0 = -2.0
        let lp_for = |tid: u32| -> f64 {
            let idx = out.token_ids.iter().position(|&t| t == tid).unwrap();
            out.log_probs[idx]
        };
        assert!((lp_for(10) - -2.0).abs() < 1e-10);
        assert!((lp_for(20) - -2.0).abs() < 1e-10);
    }

    #[test]
    fn temperature_zero_gives_argmax() {
        // Sorted descending: token 5 has highest log-prob.
        let agg = logits(&[5, 3, 1], &[-0.5, -1.0, -2.0]);
        let tok = sample_token(&agg, 0.0);
        assert_eq!(tok, 5);
    }

    #[test]
    fn empty_expert_list_returns_empty() {
        let out = aggregate_logits(vec![]);
        assert!(out.token_ids.is_empty());
    }

    #[test]
    fn truncates_to_top_256() {
        // Build an expert with 300 tokens.
        let ids: Vec<u32> = (0..300).collect();
        let lps: Vec<f64> = (0..300).map(|i| -(i as f64)).collect();
        let input = logits(&ids, &lps);
        let out = aggregate_logits(vec![(input, 1.0)]);
        assert!(out.token_ids.len() <= SparseLogits::TOP_K);
    }
}
