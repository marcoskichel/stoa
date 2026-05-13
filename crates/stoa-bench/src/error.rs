/// Errors produced by benchmark adapters and the runner.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub(crate) enum BenchError {
    /// The recall backend is not yet implemented; available after M4 lands.
    #[error("recall backend not ready — LocalChromaSqliteBackend lands in M4")]
    BackendNotReady,

    /// Required corpus data was not found at the given path.
    ///
    /// Run `just bench-download-corpus` to populate the cache.
    #[error("corpus missing at `{path}` — run `just bench-download-corpus` first")]
    CorpusMissing { path: String },

    /// Underlying I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parse or serialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The caller did not supply an output directory.
    #[error("--output <dir> is required; the bench runner writes machine-readable JSON files")]
    OutputRequired,
}
