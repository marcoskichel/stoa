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

/// Shared stub used by all pre-M4 adapters.
///
/// Validates the smoke fixture when `params.smoke` is set, then returns
/// `BackendNotReady`. Adapters swap this for real logic once
/// `LocalChromaSqliteBackend` lands in M4.
///
/// Private items in a module are accessible to all its child modules, so each
/// adapter in this directory can call this via `super::run_stub`.
fn run_stub(name: &str, params: &RunParams) -> Result<BenchmarkResult, BenchError> {
    if params.smoke {
        load_smoke_fixture(&params.corpus_dir, name)?;
    }
    Err(BenchError::BackendNotReady)
}
