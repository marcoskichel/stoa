//! Daemon-driven recall for the inject hook.
//!
//! Connects to `stoa-recalld` over its Unix socket and asks for the
//! top-K wiki hits for each query in the ladder, stopping on the first
//! non-empty hit set. The daemon is allowed to be missing — if the
//! socket is unreachable, the hook degrades to an empty injection.

use stoa_recall::{Filters, Hit, MempalaceBackend, RecallBackend};

/// Top-K hits to ask for. The token-budget cap in `wrap.rs` truncates
/// further when the snippet bodies are long.
pub(crate) const RECALL_K: usize = 8;

/// Run each query against `backend` until one returns a non-empty hit
/// set. Returns `(query, hits)`; an empty `hits` means no query in the
/// ladder produced a match (or the daemon was unreachable).
pub(crate) async fn search_first_with_hits(
    backend: &MempalaceBackend,
    ladder: &[String],
) -> (String, Vec<Hit>) {
    let primary = ladder.first().cloned().unwrap_or_default();
    if ladder.is_empty() {
        return (primary, Vec::new());
    }
    let filters = Filters::wiki_only();
    for query in ladder {
        if query.trim().is_empty() {
            continue;
        }
        match backend.search(query, RECALL_K, &filters).await {
            Ok(hits) if !hits.is_empty() => return (query.clone(), hits),
            Ok(_) => {},
            Err(e) => {
                tracing::warn!(?e, query, "inject: daemon search failed");
                return (query.clone(), Vec::new());
            },
        }
    }
    (primary, Vec::new())
}
