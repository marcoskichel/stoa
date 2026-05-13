//! `BeamAdapter`.

use async_trait::async_trait;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// BEAM adapter — recall at 128K / 500K / 1M / 10M token scale.
pub(crate) struct BeamAdapter;

#[async_trait]
impl BenchmarkAdapter for BeamAdapter {
    fn name(&self) -> &'static str {
        "beam"
    }

    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params).await
    }
}
