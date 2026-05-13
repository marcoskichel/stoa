//! `BenchmarkAdapter` trait + shared per-run parameters.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use stoa_recall::RecallBackend;

use crate::{error::BenchError, result::BenchmarkResult};

/// Parameters shared across all benchmark runs.
pub(crate) struct RunParams {
    /// Root directory that holds downloaded corpus data.
    pub(crate) corpus_dir: PathBuf,
    /// When true, load the 5-question smoke slice instead of the full corpus.
    pub(crate) smoke: bool,
    /// Backend identifier string for result metadata (e.g. `local-chroma-sqlite`).
    pub(crate) backend_name: String,
    /// Live recall backend the adapter exercises.
    pub(crate) backend: Arc<dyn RecallBackend>,
    /// Backbone LLM identifier for result metadata.
    pub(crate) backbone_model: String,
    /// Pinned scorer git revision for result metadata.
    pub(crate) scorer_rev: String,
}

/// Interface every v0.1 benchmark adapter must satisfy.
#[async_trait]
pub(crate) trait BenchmarkAdapter: Send + Sync {
    /// Short identifier that matches the benchmark's directory under `benchmarks/`.
    fn name(&self) -> &'static str;

    /// Execute the benchmark (full or smoke depending on `params.smoke`).
    async fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError>;
}

/// Path to the committed smoke fixture for a given benchmark.
///
/// Fixture lives at `benchmarks/<name>/fixtures/smoke.json` relative to the
/// corpus root's parent (i.e. `benchmarks/`).
pub(crate) fn smoke_fixture_path(corpus_dir: &Path, bench_name: &str) -> PathBuf {
    let bench_root = corpus_dir.parent().unwrap_or(corpus_dir);
    bench_root
        .join(bench_name)
        .join("fixtures")
        .join("smoke.json")
}

/// Load and parse the smoke fixture, returning the raw JSON value.
pub(crate) fn load_smoke_fixture(
    corpus_dir: &Path,
    bench_name: &str,
) -> Result<serde_json::Value, BenchError> {
    let path = smoke_fixture_path(corpus_dir, bench_name);
    if !path.exists() {
        let display = path.to_string_lossy().into_owned();
        return Err(BenchError::CorpusMissing { path: display });
    }
    let content = std::fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    Ok(value)
}
