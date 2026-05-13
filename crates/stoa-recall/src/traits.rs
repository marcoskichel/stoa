//! `RecallBackend` trait + filters + error type.
//!
//! Mempalace is the sole impl shipped with Stoa, but the trait stays
//! pluggable. Implementors are expected to be `Send + Sync + 'static`
//! and own their interior mutability (connection pool, socket etc.).

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::hit::Hit;

/// Result alias keyed off `RecallError`.
pub type RecallResult<T> = Result<T, RecallError>;

/// Inclusive equality filters passed through to the backend.
///
/// `MemPalace` understands `wing`, `room`, and any metadata key the daemon
/// stores on drawer writes (e.g. `kind=wiki` for the wiki-as-drawer
/// pattern). Unknown keys are silently ignored by the backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filters {
    /// Inclusive equality filters. Empty map = no filtering.
    #[serde(flatten)]
    pub eq: BTreeMap<String, String>,
}

impl Filters {
    /// Build a filter set from one `(key, value)` pair.
    #[must_use]
    pub fn one(key: &str, value: &str) -> Self {
        let mut eq = BTreeMap::new();
        let _previous = eq.insert(key.to_owned(), value.to_owned());
        Self { eq }
    }

    /// Wiki-only filter: `kind = "wiki"`.
    #[must_use]
    pub fn wiki_only() -> Self {
        Self::one("kind", "wiki")
    }
}

/// Errors surfaced by any [`RecallBackend`].
#[derive(Debug, Error)]
pub enum RecallError {
    /// Backend unreachable (daemon down, socket missing).
    #[error("backend unavailable: {0}")]
    Unavailable(String),

    /// Caller passed an invalid argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Underlying I/O failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON encode/decode failure on the wire.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Daemon returned a typed error response.
    #[error("daemon error [{code}]: {message}")]
    Daemon {
        /// Symbolic code emitted by the daemon (e.g. `not_found`, `palace_missing`).
        code: String,
        /// Human-readable message.
        message: String,
    },

    /// Backend exceeded the per-call deadline.
    #[error("deadline exceeded after {millis}ms")]
    DeadlineExceeded {
        /// How long the call ran before timing out.
        millis: u64,
    },
}

/// The contract for any retrieval substrate Stoa can drive.
#[async_trait]
pub trait RecallBackend: Send + Sync + 'static {
    /// Semantic + BM25 hybrid search.
    ///
    /// Returns up to `top_k` hits sorted by score descending. Empty list
    /// is a valid response (no hits is not an error).
    async fn search(&self, query: &str, top_k: usize, filters: &Filters) -> RecallResult<Vec<Hit>>;

    /// Index a transcript file or arbitrary text into the backend.
    ///
    /// Fire-and-forget from the caller's POV; the backend MAY return
    /// immediately after queueing the write.
    async fn mine(&self, source_file: &str) -> RecallResult<Vec<String>>;

    /// Write (or overwrite) a wiki page in both backend index and
    /// canonical on-disk markdown. Idempotent on `page_id`.
    async fn write_wiki(
        &self,
        page_id: &str,
        frontmatter: &serde_json::Value,
        body: &str,
    ) -> RecallResult<String>;

    /// Read a wiki page back from the canonical on-disk store.
    async fn read_wiki(&self, page_id: &str) -> RecallResult<(serde_json::Value, String)>;

    /// Liveness check. MUST complete in <500ms on a healthy backend.
    async fn health(&self) -> RecallResult<serde_json::Value>;
}
