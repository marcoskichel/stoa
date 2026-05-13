//! Run a BM25 recall query against `<workspace>/.stoa/recall.db`.
//!
//! BM25-only on purpose: the `IpcBackend` requires the Python sidecar,
//! which the inject hook MUST NOT depend on (the hook runs synchronously
//! at session start, sub-10ms p95). The sync `Bm25Backend::search_bm25`
//! is called directly to avoid building a `tokio` runtime per query.

use std::path::Path;

use stoa_recall::Hit;
use stoa_recall_local_chroma_sqlite::Bm25Backend;

/// Number of BM25 hits to ask for. Token-budget cap downstream
/// truncates further.
pub(crate) const RECALL_K: usize = 12;

/// Try each query in `ladder` in order; return the first that yields
/// at least one hit, paired with the query string that produced it.
///
/// Hard guarantee per ARCH §6.2: missing or unhealthy recall MUST
/// degrade to empty injection rather than failing the hook. We log
/// errors via `tracing::warn` but never propagate them. An empty
/// ladder, an unopenable DB, or a ladder where every query returns
/// zero hits all yield `(<best-effort query>, Vec::new())` so the
/// caller can audit the attempt and skip injection.
pub(crate) fn search_first_with_hits(db_path: &Path, ladder: &[String]) -> (String, Vec<Hit>) {
    let primary = ladder.first().cloned().unwrap_or_default();
    if ladder.is_empty() || !db_path.is_file() {
        return (primary, Vec::new());
    }
    let backend = match Bm25Backend::open(db_path) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(?e, db = %db_path.display(), "inject: open recall.db failed");
            return (primary, Vec::new());
        },
    };
    iterate_ladder(&backend, ladder, primary)
}

fn iterate_ladder(backend: &Bm25Backend, ladder: &[String], primary: String) -> (String, Vec<Hit>) {
    for query in ladder {
        if query.trim().is_empty() {
            continue;
        }
        let hits = run_search(backend, query);
        if !hits.is_empty() {
            return (query.clone(), hits);
        }
    }
    (primary, Vec::new())
}

fn run_search(backend: &Bm25Backend, query: &str) -> Vec<Hit> {
    match backend.search_bm25(query, RECALL_K) {
        Ok(hits) => hits,
        Err(e) => {
            tracing::warn!(?e, "inject: bm25 search failed");
            Vec::new()
        },
    }
}
