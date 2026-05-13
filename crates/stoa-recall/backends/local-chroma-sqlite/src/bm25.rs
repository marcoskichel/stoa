//! BM25-only `RecallBackend` over `.stoa/recall.db` (FTS5).
//!
//! Always available — no Python sidecar, no embedding model. Powers
//! `stoa init --no-embeddings`, the `stoa query --streams bm25` fast
//! path, and the IPC backend's degraded-mode fallback.
//!
//! Concurrency: one `rusqlite::Connection` guarded by a `Mutex`. Async
//! calls fan out via `spawn_blocking` so a slow FTS5 query does not
//! stall the runtime. The struct itself is `Send + Sync` so it can be
//! wrapped in `Arc<dyn RecallBackend>` and shared across worker tasks.
//!
//! `bm25()` returns NEGATIVE floats — `ORDER BY bm25(docs) ASC` for
//! most-relevant first. We invert the sign before exposing as a `score`
//! so higher = better, matching the `Hit` contract.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{Connection, params};
use stoa_recall::{Filters, Hit, RecallBackend, RecallError, Stream, StreamSet};
use thiserror::Error;

use crate::sanitize::sanitize_query;
use crate::schema::ensure_schema;

/// Errors emitted by the BM25 backend internals.
#[derive(Debug, Error)]
pub enum Bm25Error {
    /// Underlying `SQLite` failure.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Filesystem failure when ensuring `.stoa/` exists.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<Bm25Error> for RecallError {
    fn from(e: Bm25Error) -> Self {
        match e {
            Bm25Error::Io(io) => Self::Io(io),
            Bm25Error::Sqlite(s) => Self::Other(format!("sqlite: {s}")),
        }
    }
}

/// BM25-only backend backed by `.stoa/recall.db`.
///
/// `conn` is `Arc<Mutex<Connection>>` so async wrappers can clone the
/// handle into `spawn_blocking` without holding the mutex guard across
/// `await` points.
#[derive(Debug)]
pub struct Bm25Backend {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl Bm25Backend {
    /// Open or create the FTS5 + KG schema at `db_path`.
    pub fn open(db_path: &Path) -> Result<Self, Bm25Error> {
        let conn = ensure_schema(db_path)?;
        Ok(Self {
            db_path: db_path.to_path_buf(),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Path of the backing `.db` file (test introspection).
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Replace the FTS5 row for `doc_id`. Idempotent on `doc_id`.
    ///
    /// Runs DELETE + INSERT in one transaction so a panic between the
    /// two `execute` calls cannot leave the row deleted-but-not-reinserted.
    /// Eliminates the mutex-poisoning risk at this callsite — the
    /// connection is never observed in a torn state.
    pub fn upsert(
        &self,
        doc_id: &str,
        kind: &str,
        source_path: &str,
        content: &str,
    ) -> Result<(), Bm25Error> {
        run_with_conn_mut(&self.conn, |c| {
            let tx = c.transaction()?;
            tx.execute(DELETE_BY_DOC_ID, params![doc_id])?;
            tx.execute(INSERT_DOC, params![doc_id, kind, source_path, content])?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Drop all FTS5 rows for `doc_id`. No-op if absent.
    pub fn delete(&self, doc_id: &str) -> Result<(), Bm25Error> {
        run_with_conn(&self.conn, |c| {
            c.execute(DELETE_BY_DOC_ID, params![doc_id])?;
            Ok(())
        })
    }

    /// Truncate every table the backend owns (`docs`, `nodes`, `edges`)
    /// inside one transaction so a `stoa index rebuild` cannot leave
    /// the index torn between deletes and re-inserts.
    ///
    /// Required by the Layer 1 / Layer 2 invariant: rebuild must be
    /// equivalent to "delete `.stoa/recall.db` and reingest".
    pub fn truncate_all(&self) -> Result<(), Bm25Error> {
        run_with_conn(&self.conn, |c| {
            c.execute_batch(TRUNCATE_ALL)?;
            Ok(())
        })
    }

    /// Run a BM25 search and return up to `k` hits, best-first.
    ///
    /// Empty queries return an empty `Vec` (FTS5 errors on bare-empty
    /// `MATCH`). Tokens that fail FTS5's quoting rules are wrapped
    /// defensively with double quotes. `k` is clamped to `i64::MAX`
    /// rather than silently falling back to 10 when it overflows
    /// `i64`.
    pub fn search_bm25(&self, query: &str, k: usize) -> Result<Vec<Hit>, Bm25Error> {
        let safe = sanitize_query(query);
        if safe.is_empty() {
            return Ok(Vec::new());
        }
        run_with_conn(&self.conn, |c| {
            let mut stmt = c.prepare(SEARCH_SQL)?;
            let limit = clamp_k_to_i64(k);
            let mut rows = stmt.query(params![safe, limit])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push(row_to_hit(row)?);
            }
            Ok(out)
        })
    }
}

fn run_with_conn<R>(
    mu: &Arc<Mutex<Connection>>,
    f: impl FnOnce(&Connection) -> Result<R, Bm25Error>,
) -> Result<R, Bm25Error> {
    let guard = match mu.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let result = f(&guard);
    drop(guard);
    result
}

fn run_with_conn_mut<R>(
    mu: &Arc<Mutex<Connection>>,
    f: impl FnOnce(&mut Connection) -> Result<R, Bm25Error>,
) -> Result<R, Bm25Error> {
    let mut guard = match mu.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let result = f(&mut guard);
    drop(guard);
    result
}

fn row_to_hit(row: &rusqlite::Row<'_>) -> Result<Hit, rusqlite::Error> {
    let doc_id: String = row.get(0)?;
    let kind: String = row.get(1)?;
    let source_path: String = row.get(2)?;
    let snippet: String = row.get(3)?;
    let raw_score: f64 = row.get(4)?;
    let mut hit = Hit::single_stream(doc_id, -raw_score, snippet, source_path, Stream::Bm25);
    let _prev = hit
        .metadata
        .insert("kind".to_owned(), serde_json::Value::String(kind));
    Ok(hit)
}

const DELETE_BY_DOC_ID: &str = "DELETE FROM docs WHERE doc_id = ?1;";

/// Wipe every persistent table this backend owns inside one transaction.
const TRUNCATE_ALL: &str = "\
BEGIN;
DELETE FROM docs;
DELETE FROM nodes;
DELETE FROM edges;
COMMIT;";

const INSERT_DOC: &str = "\
INSERT INTO docs (doc_id, kind, source_path, content) \
VALUES (?1, ?2, ?3, ?4);";

const SEARCH_SQL: &str = "\
SELECT doc_id, kind, source_path, snippet(docs, 3, '', '', '...', 16), bm25(docs) \
  FROM docs \
 WHERE docs MATCH ?1 \
 ORDER BY bm25(docs) ASC \
 LIMIT ?2;";

#[async_trait]
impl RecallBackend for Bm25Backend {
    async fn index_page(
        &self,
        page_id: &str,
        content: &str,
        source_path: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), RecallError> {
        let kind = metadata
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("page")
            .to_owned();
        let id = page_id.to_owned();
        let path = source_path.to_owned();
        let body = content.to_owned();
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            run_with_conn_mut(&conn, |c| {
                let tx = c.transaction()?;
                tx.execute(DELETE_BY_DOC_ID, params![id])?;
                tx.execute(INSERT_DOC, params![id, kind, path, body])?;
                tx.commit()?;
                Ok(())
            })
        })
        .await
        .map_err(|e| RecallError::Other(format!("join: {e}")))??;
        Ok(())
    }

    async fn index_session(
        &self,
        _session_id: &str,
        _jsonl_path: &Path,
    ) -> Result<(), RecallError> {
        Err(RecallError::Other(
            "Bm25Backend does not own session ingest; use the workspace indexer".into(),
        ))
    }

    async fn remove(&self, doc_id: &str) -> Result<(), RecallError> {
        let id = doc_id.to_owned();
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            run_with_conn(&conn, |c| {
                c.execute(DELETE_BY_DOC_ID, params![id])?;
                Ok(())
            })
        })
        .await
        .map_err(|e| RecallError::Other(format!("join: {e}")))??;
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        k: usize,
        _filters: &Filters,
        streams: StreamSet,
    ) -> Result<Vec<Hit>, RecallError> {
        if !streams.contains(Stream::Bm25) {
            return Ok(Vec::new());
        }
        let q = query.to_owned();
        let conn = Arc::clone(&self.conn);
        let hits = tokio::task::spawn_blocking(move || -> Result<Vec<Hit>, Bm25Error> {
            search_blocking(&conn, &q, k)
        })
        .await
        .map_err(|e| RecallError::Other(format!("join: {e}")))??;
        Ok(hits)
    }

    async fn health_check(&self) -> Result<serde_json::Value, RecallError> {
        Ok(serde_json::json!({
            "backend": "bm25",
            "db_path": self.db_path.display().to_string(),
        }))
    }
}

fn search_blocking(
    conn: &Arc<Mutex<Connection>>,
    query: &str,
    k: usize,
) -> Result<Vec<Hit>, Bm25Error> {
    let safe = sanitize_query(query);
    if safe.is_empty() {
        return Ok(Vec::new());
    }
    run_with_conn(conn, |c| {
        let mut stmt = c.prepare(SEARCH_SQL)?;
        let limit = clamp_k_to_i64(k);
        let mut rows = stmt.query(params![safe, limit])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_hit(row)?);
        }
        Ok(out)
    })
}

/// Clamp `k` to `i64::MAX` so a `usize` larger than `i64::MAX`
/// (theoretical on 64-bit, plausible if a caller passes
/// `usize::MAX` as a sentinel) doesn't silently fall back to 10.
fn clamp_k_to_i64(k: usize) -> i64 {
    i64::try_from(k).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::Bm25Backend;
    use std::sync::Arc;
    use stoa_recall::{Filters, RecallBackend, Stream, StreamSet};
    use tempfile::TempDir;

    #[tokio::test]
    async fn round_trip_search_finds_indexed_doc() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("recall.db");
        let be = Arc::new(Bm25Backend::open(&db).unwrap());
        be.index_page(
            "ent-redis",
            "Redis is an in-memory cache used for sessions",
            "wiki/entities/ent-redis.md",
            &serde_json::json!({"kind": "entity"}),
        )
        .await
        .unwrap();
        let hits = be
            .search("redis", 5, &Filters::default(), StreamSet::from_slice(&[Stream::Bm25]))
            .await
            .unwrap();
        assert!(!hits.is_empty(), "indexed doc must be searchable");
        assert_eq!(hits[0].doc_id, "ent-redis");
        assert!(hits[0].streams_matched.contains(&Stream::Bm25));
    }

    #[cfg(unix)]
    #[test]
    fn refuse_symlinked_recall_db() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real.db");
        std::fs::write(&real, b"").unwrap();
        let link = tmp.path().join("recall.db");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let err = Bm25Backend::open(&link).expect_err("symlinked recall.db must be rejected");
        let msg = format!("{err}");
        assert!(msg.contains("symlink"), "expected symlink rejection diagnostic, got: {msg}");
    }

    #[tokio::test]
    async fn upsert_replaces_prior_doc() {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("recall.db");
        let be = Arc::new(Bm25Backend::open(&db).unwrap());
        for _ in 0..3 {
            be.index_page(
                "ent-redis",
                "Redis is an in-memory cache",
                "wiki/entities/ent-redis.md",
                &serde_json::json!({"kind": "entity"}),
            )
            .await
            .unwrap();
        }
        let hits = be
            .search("redis", 10, &Filters::default(), StreamSet::from_slice(&[Stream::Bm25]))
            .await
            .unwrap();
        assert_eq!(hits.len(), 1, "upsert must not duplicate; got {hits:?}");
    }
}
