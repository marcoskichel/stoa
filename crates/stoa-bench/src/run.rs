//! CLI dispatch + result emission.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use stoa_recall::RecallBackend;
use stoa_recall_local_chroma_sqlite::{IpcBackend, ensure_schema};

use crate::{
    adapter::{BenchmarkAdapter, RunParams},
    adapters::{
        AgentLeakAdapter, BeamAdapter, LongmemEvalAdapter, MemoryAgentBenchAdapter,
        MemtrackAdapter, MtebAdapter,
    },
    backends::NoopBackend,
    cli::{BackendKind, BenchmarkKind, Cli},
    error::BenchError,
    report,
    result::BenchmarkResult,
};

/// Entry point for the benchmark runner.
pub(crate) async fn run(cli: &Cli) -> Result<(), BenchError> {
    let corpus_dir = resolve_corpus_dir(cli.corpus_dir.clone());
    let backend = build_backend(cli)?;
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
fn build_backend(cli: &Cli) -> Result<Arc<dyn RecallBackend>, BenchError> {
    match cli.backend {
        BackendKind::NoMemory => Ok(Arc::new(NoopBackend)),
        BackendKind::LocalChromaSqlite => build_local_backend(cli.workspace.as_deref()),
    }
}

fn build_local_backend(workspace: Option<&Path>) -> Result<Arc<dyn RecallBackend>, BenchError> {
    let (queue_db, recall_db) = match workspace {
        Some(ws) => (ws.join(".stoa/queue.db"), ws.join(".stoa/recall.db")),
        None => fresh_tempdir_paths()?,
    };
    ensure_schema(&recall_db).map_err(|e| BenchError::Backend(e.to_string()))?;
    let backend =
        IpcBackend::open(&queue_db, &recall_db).map_err(|e| BenchError::Backend(e.to_string()))?;
    Ok(Arc::new(backend))
}

fn fresh_tempdir_paths() -> Result<(PathBuf, PathBuf), BenchError> {
    let dir = std::env::temp_dir().join(format!("stoa-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let recall_db = dir.join("recall.db");
    let queue_db = dir.join("queue.db");
    drop(std::fs::remove_file(&recall_db));
    drop(std::fs::remove_file(&queue_db));
    Ok((queue_db, recall_db))
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
    let mut out = Vec::new();
    for adapter in adapters_for(cli.bench.as_ref()) {
        out.push(adapter.run(params).await?);
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
    std::fs::write(dir.join(&filename), json)?;
    report::write_markdown(result, dir)?;
    Ok(())
}
