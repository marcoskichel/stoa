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

    /// Queue payload rejected by validation (malformed id, traversal, symlink).
    #[error("payload rejected: {0}")]
    PayloadRejected(&'static str),
}

impl Error {
    /// Short, stable label persisted into `queue_events.error_kind` when a
    /// row is dead-lettered. Stable across releases — downstream tooling
    /// (e.g. `stoa doctor` in M4+) groups DLQ rows by this label.
    #[must_use]
    pub fn classify(&self) -> &'static str {
        match self {
            Self::Queue(_) => "queue",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::PayloadField(_) => "payload-field",
            Self::PayloadRejected(_) => "payload-rejected",
        }
    }
}

/// Convenience `Result` alias.
pub type Result<T> = core::result::Result<T, Error>;
