//! Typed error for `stoa-capture`.

use thiserror::Error;

/// Errors emitted by the capture worker.
#[derive(Debug, Error)]
pub enum Error {
    /// Underlying queue / `SQLite` failure.
    #[error("queue: {0}")]
    Queue(#[from] stoa_queue::Error),

    /// Filesystem IO error (reading source, writing sessions, audit log).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization / parsing failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Queue payload missing a required field.
    #[error("payload missing field `{0}`")]
    PayloadField(&'static str),
}

/// Convenience `Result` alias.
pub type Result<T> = core::result::Result<T, Error>;
