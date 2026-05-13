//! `stoa query <q>` — hybrid recall over the indexed corpus.
//!
//! M4: dispatches through [`stoa_recall_local_chroma_sqlite::Bm25Backend`]
//! for BM25-only requests (always available) and falls back to BM25 when
//! the Python sidecar is unreachable. JSON output shape is pinned by the
//! `cli_query.rs` integration tests:
//!
//! ```json
//! {"hits": [{"doc_id": "...", "score": ..., "source_path": "...",
//!            "streams_matched": ["bm25"], "snippet": "...", "metadata": {}}]}
//! ```

use std::path::PathBuf;

use anyhow::{Context, anyhow};
use stoa_recall::{Filters, Hit, RecallBackend, Stream, StreamSet};
use stoa_recall_local_chroma_sqlite::Bm25Backend;

use crate::workspace::Workspace;

/// Dispatched from `Cli::dispatch`. Returns non-zero on workspace-missing
/// or backend failure; an empty result set is NOT an error.
pub(crate) fn run(query: &str, json: bool, streams: &[String], k: usize) -> anyhow::Result<()> {
    let ws = Workspace::current().context("locating Stoa workspace")?;
    let streamset = build_stream_set(streams)?;
    let hits = search(&ws, query, k, streamset)?;
    if json {
        emit_json(&hits)?;
    } else {
        emit_text(&hits);
    }
    Ok(())
}

fn build_stream_set(streams: &[String]) -> anyhow::Result<StreamSet> {
    if streams.is_empty() {
        return Ok(StreamSet::all());
    }
    let mut set = StreamSet::from_slice(&[]);
    for s in streams {
        let parsed = Stream::parse(s.as_str())
            .ok_or_else(|| anyhow!("unknown stream `{s}`; expected vector/bm25/graph"))?;
        set.set(parsed);
    }
    if set.is_empty() {
        return Err(anyhow!("no streams selected after parsing {streams:?}"));
    }
    Ok(set)
}

fn search(ws: &Workspace, query: &str, k: usize, streams: StreamSet) -> anyhow::Result<Vec<Hit>> {
    let db_path = recall_db_path(ws);
    let bm25 =
        Bm25Backend::open(&db_path).with_context(|| format!("opening `{}`", db_path.display()))?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    let hits = rt.block_on(async { bm25.search(query, k, &Filters::default(), streams).await });
    hits.map_err(|e| anyhow!("recall search failed: {e}"))
}

fn recall_db_path(ws: &Workspace) -> PathBuf {
    ws.root
        .join(".stoa")
        .join(stoa_recall_local_chroma_sqlite::RECALL_DB_FILE)
}

#[expect(
    clippy::print_stdout,
    reason = "CLI subcommand emits JSON to stdout by design."
)]
fn emit_json(hits: &[Hit]) -> anyhow::Result<()> {
    let payload = serde_json::json!({"hits": hits});
    let text = serde_json::to_string_pretty(&payload).context("serializing hits")?;
    println!("{text}");
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "CLI subcommand emits ranked hits to stdout by design."
)]
fn emit_text(hits: &[Hit]) {
    if hits.is_empty() {
        println!("(no hits)");
        return;
    }
    for (rank, hit) in hits.iter().enumerate() {
        println!("{:>2}. [{:.3}] {} — {}", rank + 1, hit.score, hit.doc_id, hit.source_path);
        if !hit.snippet.is_empty() {
            println!("    {}", hit.snippet);
        }
    }
}
