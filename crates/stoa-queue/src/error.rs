//! Typed error for `stoa-queue`.
//!
//! `rusqlite::Error` and `serde_json::Error` are folded into one variant
//! each; callers either propagate via `?` (workers) or `.unwrap()` (tests).

use thiserror::Error;

/// Errors emitted by `stoa-queue`.
#[derive(Debug, Error)]
pub enum Error {
    /// Underlying `SQLite` failure (open / pragma / query).
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON serialization failure when persisting a payload value.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience `Result` alias for the crate.
pub type Result<T> = core::result::Result<T, Error>;
