//! Drain one row from the `recall.request` lane.
//!
//! For M4 the only supported method is `index_page` — the daemon
//! re-indexes the named page (or every changed page if the watcher
//! enqueued the row in batch mode). Vector/KG ingestion are deferred to
//! the Python sidecar; the Rust daemon handles BM25 reindex inline so
//! single-stream queries succeed without the sidecar.

use std::path::Path;

use anyhow::{Context, anyhow};
use stoa_queue::{ClaimedRow, Queue};
use stoa_recall_local_chroma_sqlite::Bm25Backend;

use crate::index;

/// Lane the daemon claims from.
const LANE: &str = "recall.request";

/// Worker identifier (single-process; uniqued via PID).
const WORKER_PREFIX: &str = "stoa-recall-drain";

/// Lease for one drain cycle. Generous because `index_page` is bounded
/// by FTS5 insert latency (sub-second on real workspaces).
const LEASE_SECS: i64 = 60;

/// Max retry budget per row before dead-lettering.
const MAX_ATTEMPTS: i64 = 3;

/// Drain one row from `recall.request`. Returns `Ok(true)` if a row was
/// processed, `Ok(false)` if the lane was empty.
pub(crate) fn drain_one(workspace_root: &Path, queue: &Queue) -> anyhow::Result<bool> {
    let Some(row) = queue
        .claim_on_lanes(&worker_id(), LEASE_SECS, &[LANE])
        .context("claiming recall.request row")?
    else {
        return Ok(false);
    };
    handle_outcome(queue, &row, process(workspace_root, &row))
}

fn handle_outcome(
    queue: &Queue,
    row: &ClaimedRow,
    outcome: anyhow::Result<()>,
) -> anyhow::Result<bool> {
    match outcome {
        Ok(()) => {
            queue
                .complete(row.id)
                .context("marking recall.request done")?;
            Ok(true)
        },
        Err(e) => {
            record_failure(queue, row, &e)?;
            Err(e)
        },
    }
}

fn record_failure(queue: &Queue, row: &ClaimedRow, err: &anyhow::Error) -> anyhow::Result<()> {
    let outcome = queue
        .record_failure(row.id, "recall.request", MAX_ATTEMPTS)
        .context("recording recall.request failure")?;
    if outcome.dead_lettered {
        tracing::error!(
            row_id = row.id,
            attempts = outcome.attempts,
            error = %err,
            "recall.request dead-lettered",
        );
    }
    Ok(())
}

fn process(workspace_root: &Path, row: &ClaimedRow) -> anyhow::Result<()> {
    let payload: serde_json::Value = serde_json::from_str(&row.payload)
        .with_context(|| format!("parsing recall.request payload: {}", row.payload))?;
    let method = payload
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("recall.request missing `method`"))?;
    let args = payload
        .get("args")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match method {
        "index_page" => index_one_page(workspace_root, &args),
        other => Err(anyhow!("unsupported recall.request method `{other}`")),
    }
}

fn index_one_page(workspace_root: &Path, args: &serde_json::Value) -> anyhow::Result<()> {
    let path_rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("recall.request.index_page missing `path`"))?;
    let abs = workspace_root.join(path_rel);
    if !abs.is_file() {
        index::reindex_via_full_rebuild(workspace_root)?;
        return Ok(());
    }
    let bm25 = open_bm25(workspace_root)?;
    index::reindex_one_wiki_page(&abs, &bm25, path_rel)
}

fn open_bm25(workspace_root: &Path) -> anyhow::Result<Bm25Backend> {
    let db = workspace_root
        .join(".stoa")
        .join(stoa_recall_local_chroma_sqlite::RECALL_DB_FILE);
    Bm25Backend::open(&db).with_context(|| format!("opening `{}`", db.display()))
}

fn worker_id() -> String {
    format!("{WORKER_PREFIX}-{}", std::process::id())
}
