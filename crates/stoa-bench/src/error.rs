//! Errors produced by the benchmark runner and its adapters.

/// Errors produced by benchmark adapters and the runner.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub(crate) enum BenchError {
    /// The recall backend is not yet implemented for this surface.
    #[error("recall backend not ready — wiring lands in a follow-up")]
    BackendNotReady,

    /// Required corpus data was not found at the given path.
    #[error("corpus missing at `{path}` — run `just bench-download-corpus` first")]
    CorpusMissing { path: String },

    /// Underlying I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parse or serialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// `--output <dir>` was not supplied.
    #[error("--output <dir> is required; the bench runner writes machine-readable JSON files")]
    OutputRequired,

    /// Underlying recall backend failed.
    #[error("backend error: {0}")]
    Backend(String),

    /// Recall returned an error.
    #[error("recall error: {0}")]
    Recall(#[from] stoa_recall::RecallError),

    /// Corpus parse error with explanation.
    #[error("corpus parse error: {0}")]
    CorpusParse(String),
}
