//! Per-benchmark adapter implementations.

mod agent_leak;
mod beam;
mod longmemeval;
mod memory_agent_bench;
mod memtrack;
mod mteb;

pub(crate) use agent_leak::AgentLeakAdapter;
pub(crate) use beam::BeamAdapter;
pub(crate) use longmemeval::LongmemEvalAdapter;
pub(crate) use memory_agent_bench::MemoryAgentBenchAdapter;
pub(crate) use memtrack::MemtrackAdapter;
pub(crate) use mteb::MtebAdapter;

use crate::{
    adapter::{RunParams, load_smoke_fixture},
    error::BenchError,
    result::BenchmarkResult,
};

/// Shared stub used by adapters that don't yet have a real implementation.
///
/// Validates the smoke fixture when `params.smoke` is set, then returns
/// `BackendNotReady`. Replaced by real logic adapter-by-adapter.
async fn run_stub(name: &str, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
    if params.smoke {
        let _value = load_smoke_fixture(&params.corpus_dir, name)?;
    }
    Err(BenchError::BackendNotReady)
}
