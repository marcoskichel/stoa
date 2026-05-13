use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// `LongMemEval` adapter — five-category multi-session recall + reasoning.
///
/// Source: Wu et al. 2024, <https://arxiv.org/abs/2410.10813>.
/// Corpus: `xiaowu0162/longmemeval` on `HuggingFace`.
/// Metrics: `recall@1`, `recall@5`, `recall@10` per category.
/// Cost: ~$30–60 per full run at Haiku rates.
pub(crate) struct LongmemEvalAdapter;

impl BenchmarkAdapter for LongmemEvalAdapter {
    fn name(&self) -> &'static str {
        "longmemeval"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
