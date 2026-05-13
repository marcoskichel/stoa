use std::path::{Path, PathBuf};

use crate::{error::BenchError, result::BenchmarkResult};

/// Parameters shared across all benchmark runs.
///
/// Expanded in M4 to carry backend, version, `backbone_model`, and `scorer_rev`
/// once adapters produce real `BenchmarkResult` values.
#[derive(Debug, Clone)]
pub(crate) struct RunParams {
    /// Root directory that holds downloaded corpus data.
    pub(crate) corpus_dir: PathBuf,
    /// When true, run the 5-question smoke slice instead of the full corpus.
    pub(crate) smoke: bool,
}

/// Interface every v0.1 benchmark adapter must satisfy.
pub(crate) trait BenchmarkAdapter: Send + Sync {
    /// Short identifier that matches the benchmark's directory under `benchmarks/`.
    fn name(&self) -> &'static str;

    /// Execute the benchmark (full or smoke depending on `params.smoke`).
    ///
    /// Returns `Err(BenchError::BackendNotReady)` until `LocalChromaSqliteBackend`
    /// lands in M4.
    fn run(&self, params: &RunParams) -> Result<BenchmarkResult, BenchError>;
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
///
/// Fails with `CorpusMissing` if the file does not exist, `Json` if it is
/// invalid JSON.
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
