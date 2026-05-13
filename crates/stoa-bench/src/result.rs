use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Fully-provenance-tagged result for one benchmark run.
///
/// Written to `results/<version>-<backend>-<benchmark>.json` by CI.
/// Manual edits are forbidden — see `benchmarks/results/README.md`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BenchmarkResult {
    /// Benchmark identifier (matches directory name under `benchmarks/`).
    pub(crate) benchmark: String,
    /// Backend identifier (matches `BackendKind::as_str()`).
    pub(crate) backend: String,
    /// Stoa crate version at run time.
    pub(crate) version: String,
    /// Pinned corpus git revision for reproducibility.
    pub(crate) corpus_rev: String,
    /// Pinned upstream scorer git revision.
    pub(crate) scorer_rev: String,
    /// Backbone LLM used for generation and judging.
    pub(crate) backbone_model: String,
    /// All hyperparameters that influence the result (k, streams, thresholds).
    pub(crate) hyperparams: BTreeMap<String, serde_json::Value>,
    /// Primary metrics (e.g. `recall@1`, `recall@5`, accuracy).
    pub(crate) metrics: BTreeMap<String, f64>,
    /// Estimated API cost for this run in USD.
    pub(crate) cost_usd: f64,
    /// Total tokens consumed across generation + judging.
    pub(crate) tokens_used: u64,
    /// Wall-clock seconds for the full run.
    pub(crate) wall_seconds: u64,
    /// UTC timestamp of the run.
    pub(crate) timestamp: DateTime<Utc>,
}
