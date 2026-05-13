/// Errors produced by recall backends.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecallError {
    /// Backend has not been implemented yet.
    #[error("backend not yet implemented")]
    NotImplemented,
    /// The derived index is absent; run `stoa index rebuild` to regenerate it.
    #[error("index is absent — run `stoa index rebuild` first")]
    IndexMissing,
    /// Underlying I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
