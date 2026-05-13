//! Drain one row from the `recall.request` lane.
//!
//! The Rust daemon owns BM25-side reindex for `index_page` and
//! `remove_page` so single-stream queries succeed even when the
//! Python sidecar is down. Vector / KG ingest are owned by the
//! sidecar; rows of those methods are acked separately on the same
//! lane by the Python worker.
//!
//! Read-side `search` rows live on a different lane
//! ([`stoa_recall_local_chroma_sqlite::SEARCH_LANE`]) and are never
//! claimed here — splitting reads from writes is what prevents a
//! claim → release livelock when the sidecar is offline.

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
    match process(workspace_root, &row) {
        Ok(()) => {
            queue
                .complete(row.id)
                .context("marking recall.request done")?;
            Ok(true)
        },
        Err(e) => {
            record_failure(queue, &row, &e)?;
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
        "remove_page" => remove_one_page(workspace_root, &args),
        other => Err(anyhow!(
            "recall.request: unknown method `{other}` (search lives on `recall.search`)"
        )),
    }
}

fn remove_one_page(workspace_root: &Path, args: &serde_json::Value) -> anyhow::Result<()> {
    let page_id = args
        .get("page_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("recall.request.remove_page missing `page_id`"))?;
    let bm25 = open_bm25(workspace_root)?;
    bm25.delete(page_id)
        .map_err(|e| anyhow!("delete `{page_id}`: {e}"))
}

fn index_one_page(workspace_root: &Path, args: &serde_json::Value) -> anyhow::Result<()> {
    let path_rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("recall.request.index_page missing `path`"))?;
    validate_workspace_relative(path_rel)?;
    let abs = workspace_root.join(path_rel);
    let abs = canonicalize_inside(workspace_root, &abs)?;
    if !abs.is_file() {
        index::reindex_via_full_rebuild(workspace_root)?;
        return Ok(());
    }
    let bm25 = open_bm25(workspace_root)?;
    index::reindex_one_wiki_page(&abs, &bm25, path_rel)
}

/// Reject obviously-hostile payloads before touching the filesystem.
///
/// Forbids parent-segment escapes (`..`), absolute paths (leading `/`
/// or Windows drive letters), and embedded NUL bytes that would split a
/// `CString`. A path that survives this check still has to canonicalize
/// inside the workspace root in [`canonicalize_inside`].
fn validate_workspace_relative(path_rel: &str) -> anyhow::Result<()> {
    if path_rel.is_empty() {
        return Err(anyhow!("recall.request.index_page `path` is empty"));
    }
    if path_rel.contains('\0') {
        return Err(anyhow!("recall.request.index_page `path` contains NUL byte"));
    }
    let pb = std::path::PathBuf::from(path_rel);
    if pb.is_absolute() {
        return Err(anyhow!("recall.request.index_page `path` must be workspace-relative"));
    }
    for component in pb.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(anyhow!("recall.request.index_page `path` may not contain `..`"));
        }
    }
    Ok(())
}

/// Canonicalize `candidate` and assert the result lives under
/// `workspace_root` (also canonicalized).
///
/// `candidate` may not exist yet — in that case we canonicalize the
/// nearest existing ancestor and re-attach the trailing components.
fn canonicalize_inside(
    workspace_root: &Path,
    candidate: &Path,
) -> anyhow::Result<std::path::PathBuf> {
    let root_canon = workspace_root
        .canonicalize()
        .with_context(|| format!("canonicalizing workspace root `{}`", workspace_root.display()))?;
    let candidate_canon = canonicalize_lenient(candidate)?;
    if !candidate_canon.starts_with(&root_canon) {
        return Err(anyhow!(
            "recall.request.index_page path `{}` escapes workspace root `{}`",
            candidate_canon.display(),
            root_canon.display(),
        ));
    }
    Ok(candidate_canon)
}

/// Canonicalize `path` even if it does not exist yet by canonicalizing
/// the longest existing ancestor and re-appending the missing tail.
fn canonicalize_lenient(path: &Path) -> anyhow::Result<std::path::PathBuf> {
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path.to_path_buf();
    while let Some(parent) = cursor.parent() {
        if let Some(name) = cursor.file_name() {
            tail.push(name.to_os_string());
        }
        if let Ok(parent_canon) = parent.canonicalize() {
            let mut acc = parent_canon;
            for segment in tail.iter().rev() {
                acc.push(segment);
            }
            return Ok(acc);
        }
        cursor = parent.to_path_buf();
    }
    Err(anyhow!(
        "could not canonicalize `{}` against any existing ancestor",
        path.display()
    ))
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
