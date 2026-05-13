use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// MTEB/BEIR subset adapter — embedding component quality check.
///
/// Validates the vector retrieval component in isolation using a representative
/// BEIR sub-corpus. Internal engineering gate — does not block v0.1 release
/// if absent, but provides signal on embedding model quality before full recall
/// benchmarks run.
/// Cost: near-zero (no backbone inference required).
pub(crate) struct MtebAdapter;

impl BenchmarkAdapter for MtebAdapter {
    fn name(&self) -> &'static str {
        "mteb-retrieval"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
