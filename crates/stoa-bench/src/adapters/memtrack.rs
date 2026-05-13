use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// MEMTRACK adapter — multi-platform event-timeline state tracking.
///
/// 47 expert-curated scenarios across Slack, Linear, and Git. Exercises
/// conflict resolution and temporal ordering. No other memory vendor has
/// published on this benchmark, making it a strong first-mover position.
/// Cost: very low (no judge LLM required for most scenarios).
pub(crate) struct MemtrackAdapter;

impl BenchmarkAdapter for MemtrackAdapter {
    fn name(&self) -> &'static str {
        "memtrack"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
