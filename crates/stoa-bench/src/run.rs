use std::path::{Path, PathBuf};

use stoa_recall::RecallBackend;

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    adapters::{
        AgentLeakAdapter, BeamAdapter, LongmemEvalAdapter, MemoryAgentBenchAdapter,
        MemtrackAdapter, MtebAdapter,
    },
    backends::NoopBackend,
    cli::{BackendKind, BenchmarkKind, Cli},
    error::BenchError,
    result::BenchmarkResult,
};

/// Entry point for the benchmark runner.
pub(crate) fn run(cli: &Cli) -> Result<(), BenchError> {
    let corpus_dir = resolve_corpus_dir(cli.corpus_dir.clone());
    let _backend = build_backend(&cli.backend);
    let params = build_params(cli, corpus_dir);
    let results = collect_results(cli, &params)?;
    write_results(results, cli.output.as_deref())
}

fn resolve_corpus_dir(override_: Option<PathBuf>) -> PathBuf {
    override_.unwrap_or_else(|| PathBuf::from("benchmarks/corpus"))
}

/// Instantiates the recall backend for the given CLI flag.
///
/// Both variants map to `NoopBackend` pre-M4; `LocalChromaSqlite` will wire
/// to the real backend once `LocalChromaSqliteBackend` lands.
fn build_backend(kind: &BackendKind) -> Box<dyn RecallBackend> {
    // FIXME: LocalChromaSqlite arm wires to LocalChromaSqliteBackend in M4
    match kind {
        BackendKind::LocalChromaSqlite | BackendKind::NoMemory => Box::new(NoopBackend),
    }
}

fn build_params(cli: &Cli, corpus_dir: PathBuf) -> RunParams {
    RunParams { corpus_dir, smoke: cli.smoke }
}

fn collect_results(cli: &Cli, params: &RunParams) -> Result<Vec<BenchmarkResult>, BenchError> {
    adapters_for(cli.bench.as_ref())
        .into_iter()
        .map(|a| a.run(params))
        .collect()
}

fn adapters_for(bench: Option<&BenchmarkKind>) -> Vec<Box<dyn BenchmarkAdapter>> {
    match bench {
        Some(kind) => vec![adapter_for(kind)],
        None => all_adapters(),
    }
}

fn adapter_for(kind: &BenchmarkKind) -> Box<dyn BenchmarkAdapter> {
    match kind {
        BenchmarkKind::Longmemeval => Box::new(LongmemEvalAdapter),
        BenchmarkKind::MemoryAgentBench => Box::new(MemoryAgentBenchAdapter),
        BenchmarkKind::Memtrack => Box::new(MemtrackAdapter),
        BenchmarkKind::Beam => Box::new(BeamAdapter),
        BenchmarkKind::AgentLeak => Box::new(AgentLeakAdapter),
        BenchmarkKind::Mteb => Box::new(MtebAdapter),
    }
}

fn all_adapters() -> Vec<Box<dyn BenchmarkAdapter>> {
    vec![
        Box::new(LongmemEvalAdapter),
        Box::new(MemoryAgentBenchAdapter),
        Box::new(MemtrackAdapter),
        Box::new(BeamAdapter),
        Box::new(AgentLeakAdapter),
        Box::new(MtebAdapter),
    ]
}

/// Write all results to JSON files in `output`.
///
/// Requires `--output <dir>` — the bench runner is primarily a CI tool and
/// produces machine-readable files rather than streaming to stdout.
fn write_results(results: Vec<BenchmarkResult>, output: Option<&Path>) -> Result<(), BenchError> {
    let dir = output.ok_or(BenchError::OutputRequired)?;
    for result in results {
        let json = serde_json::to_string_pretty(&result)?;
        write_to_file(&result, &json, dir)?;
    }
    Ok(())
}

fn write_to_file(result: &BenchmarkResult, json: &str, dir: &Path) -> Result<(), BenchError> {
    let filename = format!("{}-{}-{}.json", result.version, result.backend, result.benchmark);
    std::fs::write(dir.join(filename), json)?;
    Ok(())
}
