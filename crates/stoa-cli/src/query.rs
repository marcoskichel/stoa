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

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use chrono::{SecondsFormat, Utc};
use stoa_recall::{Filters, Hit, RecallBackend, Stream, StreamSet};
use stoa_recall_local_chroma_sqlite::{Bm25Backend, IpcBackend};

use crate::workspace::Workspace;

/// Dispatched from `Cli::dispatch`. Returns non-zero on workspace-missing
/// or backend failure; an empty result set is NOT an error.
pub(crate) fn run(query: &str, json: bool, streams: &[String], k: usize) -> anyhow::Result<()> {
    let ws = Workspace::current().context("locating Stoa workspace")?;
    let streamset = build_stream_set(streams)?;
    let hits = search(&ws, query, k, streamset)?;
    let _ignored = audit_log_query(&ws, query, k, streamset, hits.len());
    if json {
        emit_json(&hits)?;
    } else {
        emit_text(&hits);
    }
    Ok(())
}

/// Append one structured row to `.stoa/audit.log` per ARCHITECTURE §10.
///
/// Failures here MUST NOT fail the user-facing query — the audit write
/// is a best-effort observability hook. Callers swallow the result.
fn audit_log_query(
    ws: &Workspace,
    query: &str,
    k: usize,
    streams: StreamSet,
    hit_count: usize,
) -> std::io::Result<()> {
    let log_path = ws.root.join(".stoa").join("audit.log");
    let line = build_audit_line(query, k, streams, hit_count);
    append_audit_line(&log_path, &line)
}

fn build_audit_line(query: &str, k: usize, streams: StreamSet, hit_count: usize) -> String {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let stream_names: Vec<&str> = streams.iter().map(Stream::as_str).collect();
    let entry = serde_json::json!({
        "ts": ts,
        "event": "stoa.query",
        "query": query,
        "k": k,
        "streams": stream_names,
        "hits": hit_count,
    });
    format!("{entry}\n")
}

fn append_audit_line(log_path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    f.write_all(line.as_bytes())
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
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    if embeddings_enabled(ws) {
        let backend = open_ipc_backend(ws)?;
        rt.block_on(async { backend.search(query, k, &Filters::default(), streams).await })
            .map_err(|e| anyhow!("recall search failed: {e}"))
    } else {
        let backend = open_bm25_backend(ws)?;
        rt.block_on(async { backend.search(query, k, &Filters::default(), streams).await })
            .map_err(|e| anyhow!("recall search failed: {e}"))
    }
}

/// Workspace has embeddings enabled iff `.stoa/vectors/` exists. The
/// directory is created (or skipped) by `stoa init`; absence means the
/// user opted into `--no-embeddings` so we stick to BM25-only.
fn embeddings_enabled(ws: &Workspace) -> bool {
    ws.root.join(".stoa").join("vectors").is_dir()
}

fn open_bm25_backend(ws: &Workspace) -> anyhow::Result<Bm25Backend> {
    let db_path = recall_db_path(ws);
    Bm25Backend::open(&db_path).with_context(|| format!("opening `{}`", db_path.display()))
}

fn open_ipc_backend(ws: &Workspace) -> anyhow::Result<IpcBackend> {
    let db_path = recall_db_path(ws);
    let queue_path = ws.root.join(".stoa").join("queue.db");
    IpcBackend::open(&queue_path, &db_path)
        .with_context(|| format!("opening IPC backend at `{}`", db_path.display()))
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
