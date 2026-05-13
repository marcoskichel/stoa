//! CLI dispatch + result emission.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use stoa_recall::RecallBackend;
use stoa_recall_local_chroma_sqlite::{Bm25Backend, ensure_schema};

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
pub(crate) async fn run(cli: &Cli) -> Result<(), BenchError> {
    let corpus_dir = resolve_corpus_dir(cli.corpus_dir.clone());
    let backend = build_backend(&cli.backend)?;
    let params = build_params(cli, corpus_dir, backend);
    let results = collect_results(cli, &params).await?;
    write_results(results, cli.output.as_deref())
}

fn resolve_corpus_dir(override_: Option<PathBuf>) -> PathBuf {
    override_.unwrap_or_else(|| PathBuf::from("benchmarks/corpus"))
}

/// Instantiate the recall backend for the given CLI flag.
///
/// `LocalChromaSqlite` here resolves to the BM25 leg of the M4 backend
/// (the IPC leg requires the Python sidecar running). BM25 alone covers
/// the smoke path and is the M4 default for `--no-embeddings` workspaces.
fn build_backend(kind: &BackendKind) -> Result<Arc<dyn RecallBackend>, BenchError> {
    match kind {
        BackendKind::NoMemory => Ok(Arc::new(NoopBackend)),
        BackendKind::LocalChromaSqlite => build_bm25_backend(),
    }
}

fn build_bm25_backend() -> Result<Arc<dyn RecallBackend>, BenchError> {
    let tmp = std::env::temp_dir().join("stoa-bench-recall.db");
    let _result = std::fs::remove_file(&tmp);
    ensure_schema(&tmp).map_err(|e| BenchError::Backend(e.to_string()))?;
    let backend = Bm25Backend::open(&tmp).map_err(|e| BenchError::Backend(e.to_string()))?;
    Ok(Arc::new(backend))
}

fn build_params(cli: &Cli, corpus_dir: PathBuf, backend: Arc<dyn RecallBackend>) -> RunParams {
    RunParams {
        corpus_dir,
        smoke: cli.smoke,
        backend_name: cli.backend.as_str().to_owned(),
        backend,
        backbone_model: cli.backbone_model.clone(),
        scorer_rev: cli
            .scorer_rev
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
    }
}

async fn collect_results(
    cli: &Cli,
    params: &RunParams,
) -> Result<Vec<BenchmarkResult>, BenchError> {
    let single = cli.bench.is_some();
    let mut out = Vec::new();
    for adapter in adapters_for(cli.bench.as_ref()) {
        match adapter.run(params).await {
            Ok(result) => out.push(result),
            Err(BenchError::BackendNotReady) if !single => (),
            Err(other) => return Err(other),
        }
    }
    Ok(out)
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

fn write_results(results: Vec<BenchmarkResult>, output: Option<&Path>) -> Result<(), BenchError> {
    let dir = output.ok_or(BenchError::OutputRequired)?;
    std::fs::create_dir_all(dir)?;
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
