//! `MemtrackAdapter`.

use async_trait::async_trait;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// MEMTRACK adapter — multi-platform event-timeline state tracking.
pub(crate) struct MemtrackAdapter;

#[async_trait]
impl BenchmarkAdapter for MemtrackAdapter {
    fn name(&self) -> &'static str {
        "memtrack"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params).await
    }
}
