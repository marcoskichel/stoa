use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// BEAM adapter — recall at 128K / 500K / 1M / 10M token scale.
///
/// Source: ICLR 2026, <https://arxiv.org/abs/2510.27246>.
/// The 10M tier is the differentiator: not yet saturated and physically
/// impossible for context-stuffing approaches. Headline metric for Stoa's
/// marketing is 1M and 10M, not 128K (already saturated by strong backbones).
/// Peers published: Hindsight 73.9% (1M), Mem0 48.6% (10M).
pub(crate) struct BeamAdapter;

impl BenchmarkAdapter for BeamAdapter {
    fn name(&self) -> &'static str {
        "beam"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
