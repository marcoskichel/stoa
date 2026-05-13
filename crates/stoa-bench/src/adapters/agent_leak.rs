//! `AgentLeakAdapter`.

use async_trait::async_trait;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// `AgentLeak` adapter — PII leak across 7 channel classes.
pub(crate) struct AgentLeakAdapter;

#[async_trait]
impl BenchmarkAdapter for AgentLeakAdapter {
    fn name(&self) -> &'static str {
        "agent-leak"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params).await
    }
}
