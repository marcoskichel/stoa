//! `MtebAdapter`.

use async_trait::async_trait;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// MTEB/BEIR subset adapter — embedding component quality check.
pub(crate) struct MtebAdapter;

#[async_trait]
impl BenchmarkAdapter for MtebAdapter {
    fn name(&self) -> &'static str {
        "mteb-retrieval"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params).await
    }
}
