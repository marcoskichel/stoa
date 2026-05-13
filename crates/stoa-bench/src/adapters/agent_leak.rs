use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    error::BenchError,
    result::BenchmarkResult,
};

use super::run_stub;

/// `AgentLeak` adapter — PII leak taxonomy across 7 channel classes.
///
/// 32 PII classes, 7 channels including C1 (output), C4 (audit log), and
/// C5 (shared-memory). C4 requires M3 capture pipeline; C5 requires M5
/// injection wrapping. Full channel coverage available after M5.
/// No other memory vendor has published on this benchmark.
pub(crate) struct AgentLeakAdapter;

impl BenchmarkAdapter for AgentLeakAdapter {
    fn name(&self) -> &'static str {
        "agent-leak"
    }

    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
        run_stub(self.name(), params)
    }
}
