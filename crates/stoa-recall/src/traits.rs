//! `RecallBackend` trait + filters + error type.
//!
//! Async + `Send + Sync + 'static` so backends can be wrapped in
//! `Arc<dyn RecallBackend<Error = ...>>` and shared across worker tasks.
//! Trait methods are `&self` because backends own their own interior
//! mutability (`Mutex<Connection>`, IPC client pool, etc.).

use std::collections::BTreeMap;
use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::hit::Hit;
use crate::stream::StreamSet;

/// Convenience `Result` alias keyed off the trait's associated `Error`.
pub type RecallResult<T, E> = Result<T, E>;

/// Filters applied to a `search` call (kind, type, time window, etc.).
///
/// A free-form `BTreeMap<String, String>` so backends can interpret keys
/// they understand and silently ignore the rest. Wire-compatible with the
/// JSON shape the Python sidecar emits.
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
}

/// Errors any [`RecallBackend`] may surface.
#[derive(Debug, Error)]
pub enum RecallError {
    /// Backend is unhealthy (Python sidecar down, queue unreachable).
    #[error("backend unavailable: {0}")]
    Unavailable(String),

    /// Caller passed an invalid argument (empty stream set, bad path, ...).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Underlying I/O failure (queue, FTS5, `ChromaDB`).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization failure (request/response payloads).
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Backend exceeded the per-call deadline.
    #[error("deadline exceeded after {millis}ms")]
    DeadlineExceeded {
        /// How long the call ran before timing out.
        millis: u64,
    },

    /// Catch-all for backend-specific failures with a descriptive message.
    #[error("backend error: {0}")]
    Other(String),
}

/// The contract every recall backend implements.
///
/// `index_page` / `index_session` are write paths — they update both the
/// vector store + BM25 index in one logical step. `search` is the read
/// path; it MUST honor the [`StreamSet`] (e.g. BM25-only must skip vector
/// store calls so the backend can answer without the embedding model).
#[async_trait]
pub trait RecallBackend: Send + Sync + 'static {
    /// Index (or re-index) a single wiki page.
    ///
    /// Idempotent on `page_id`: re-indexing replaces the prior entry.
    async fn index_page(
        &self,
        page_id: &str,
        content: &str,
        source_path: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), RecallError>;

    /// Index (or re-index) a session JSONL file. Each line is one turn.
    async fn index_session(&self, session_id: &str, jsonl_path: &Path) -> Result<(), RecallError>;

    /// Drop a doc from every stream. Idempotent.
    async fn remove(&self, doc_id: &str) -> Result<(), RecallError>;

    /// Hybrid search across the requested streams.
    ///
    /// Empty results are NOT an error — return an empty `Vec`.
    async fn search(
        &self,
        query: &str,
        k: usize,
        filters: &Filters,
        streams: StreamSet,
    ) -> Result<Vec<Hit>, RecallError>;

    /// Liveness check. Returns shape-free JSON the caller can log; the
    /// trait only requires that the call succeed in <500 ms when healthy.
    async fn health_check(&self) -> Result<serde_json::Value, RecallError>;
}
