use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// `MemoryAgentBench` adapter — selective forgetting, test-time learning, retrieval.
///
/// The `FactConsolidation` sub-task directly exercises crystallize/supersession,
/// making this the primary benchmark for validating the harvest loop.
/// Peers that have published: Mem0, Zep, Letta, MIRIX, Cognee.
/// Cost: low (rule-based scoring for most sub-tasks).
pub(crate) struct MemoryAgentBenchAdapter;

impl BenchmarkAdapter for MemoryAgentBenchAdapter {
    fn name(&self) -> &'static str {
        "memory-agent-bench"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
