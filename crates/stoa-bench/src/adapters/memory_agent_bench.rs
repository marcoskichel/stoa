//! `MemoryAgentBenchAdapter`.

use async_trait::async_trait;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// `MemoryAgentBench` adapter — selective forgetting, test-time learning, retrieval.
pub(crate) struct MemoryAgentBenchAdapter;

#[async_trait]
impl BenchmarkAdapter for MemoryAgentBenchAdapter {
    fn name(&self) -> &'static str {
        "memory-agent-bench"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params).await
    }
}
