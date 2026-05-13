//! BEIR ranking metrics — NDCG@10 + recall@k.
//!
//! Mirrors the upstream BEIR `EvaluateRetrieval` helper: graded relevance is
//! collapsed to a binary indicator for recall, and ideal-DCG is computed from
//! the sorted gain vector so DCG ≤ IDCG for every query.

use std::collections::HashMap;

/// Recall cut-offs reported for every BEIR run.
pub(super) const K_VALUES: [usize; 3] = [1, 10, 100];
/// Cut-off for the primary NDCG metric.
pub(super) const NDCG_K: usize = 10;

/// `recall@k` — fraction of relevant docs (qrel > 0) recovered in the top-`k`.
///
/// Returns `0.0` when no qrels record relevance, matching BEIR's convention
/// (a query with no positive judgements does not raise recall).
pub(super) fn recall_at_k(retrieved: &[String], qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let relevant: usize = qrels.values().filter(|&&v| v > 0).count();
    if relevant == 0 {
        return 0.0;
    }
    let hits: usize = retrieved
        .iter()
        .take(k)
        .filter(|did| qrels.get(*did).copied().unwrap_or(0) > 0)
        .count();
    precision_safe_div(hits, relevant)
}

/// Normalised discounted cumulative gain at rank `k` using graded relevance.
pub(super) fn ndcg_at_k(retrieved: &[String], qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let mut dcg = 0.0;
    for (rank, doc_id) in retrieved.iter().take(k).enumerate() {
        let rel = f64::from(qrels.get(doc_id).copied().unwrap_or(0));
        if rel > 0.0 {
            dcg += rel / log2_rank(rank);
        }
    }
    let idcg = ideal_dcg(qrels, k);
    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

fn ideal_dcg(qrels: &HashMap<String, u32>, k: usize) -> f64 {
    let mut rels: Vec<u32> = qrels.values().copied().collect();
    rels.sort_unstable_by(|a, b| b.cmp(a));
    rels.iter()
        .take(k)
        .enumerate()
        .map(|(rank, r)| f64::from(*r) / log2_rank(rank))
        .sum()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "rank index is small; precision loss is irrelevant for DCG denominator"
)]
fn log2_rank(rank: usize) -> f64 {
    ((rank + 2) as f64).log2()
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hit + total counts bounded by corpus size; well under 2^52"
)]
pub(super) fn precision_safe_div(num: usize, denom: usize) -> f64 {
    num as f64 / denom as f64
}
