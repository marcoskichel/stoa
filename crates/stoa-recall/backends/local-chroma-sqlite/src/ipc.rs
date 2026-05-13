//! IPC backend — proxies recall calls to the Python sidecar via the queue.
//!
//! Lanes:
//!
//! - `recall.request` — Rust → Python. `event` names the method
//!   (`recall.search`, `recall.index_page`, `recall.health_check`, ...);
//!   payload carries `{method, args, deadline_unix_ms}`.
//! - `recall.response` — Python → Rust. `session_id` is the original
//!   `request_id`; payload is `{ok, result, error?}`.
//!
//! For BM25-only requests we bypass IPC entirely and call
//! [`crate::Bm25Backend`] directly. Hybrid (vector / graph) requests go
//! over IPC; if the sidecar does not respond before `deadline_ms` we
//! degrade to BM25-only and emit a tracing warning.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use stoa_queue::Queue;
use stoa_recall::{Filters, Hit, RecallBackend, RecallError, Stream, StreamSet};
use uuid::Uuid;

use crate::bm25::Bm25Backend;

/// Lane the Rust caller writes onto. Python `stoa-recall.worker` claims
/// rows from this lane.
pub const REQUEST_LANE: &str = "recall.request";

/// Lane the Python sidecar writes onto with the result.
pub const RESPONSE_LANE: &str = "recall.response";

const SEARCH_EVENT: &str = "recall.search";
const INDEX_PAGE_EVENT: &str = "recall.index_page";
const REMOVE_EVENT: &str = "recall.remove";
const HEALTH_EVENT: &str = "recall.health_check";

const DEFAULT_SEARCH_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_INDEX_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_HEALTH_TIMEOUT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// IPC-backed `RecallBackend`. Holds an `Arc<Queue>` for cheap clones
/// across async tasks + an `Arc<Bm25Backend>` for the fallback path.
#[derive(Debug)]
pub struct IpcBackend {
    queue: Arc<Queue>,
    bm25: Arc<Bm25Backend>,
    queue_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct RequestPayload<'a> {
    method: &'a str,
    args: serde_json::Value,
    deadline_unix_ms: u128,
}

#[derive(Debug, Deserialize)]
struct ResponsePayload {
    ok: bool,
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    error: Option<ResponseError>,
}

#[derive(Debug, Deserialize)]
struct ResponseError {
    #[serde(default)]
    msg: String,
}

impl IpcBackend {
    /// Open both the queue and a BM25 fallback against the given paths.
    pub fn open(queue_path: &Path, recall_db: &Path) -> Result<Self, RecallError> {
        let queue =
            Queue::open(queue_path).map_err(|e| RecallError::Other(format!("queue open: {e}")))?;
        let bm25 = Bm25Backend::open(recall_db)
            .map_err(|e| RecallError::Other(format!("bm25 open: {e}")))?;
        Ok(Self {
            queue: Arc::new(queue),
            bm25: Arc::new(bm25),
            queue_path: queue_path.to_path_buf(),
        })
    }

    /// Direct access to the BM25 backend (used by the workspace indexer).
    #[must_use]
    pub fn bm25(&self) -> Arc<Bm25Backend> {
        Arc::clone(&self.bm25)
    }

    /// Path to the queue DB this backend is bound to.
    #[must_use]
    pub fn queue_path(&self) -> &Path {
        &self.queue_path
    }

    /// Send `payload` on the request lane, await the response, return the
    /// raw `result` JSON.
    async fn round_trip(
        &self,
        event: &'static str,
        method: &'static str,
        args: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, RecallError> {
        let request_id = Uuid::new_v4().to_string();
        let deadline_ms = unix_ms_now().saturating_add(timeout.as_millis());
        let payload = RequestPayload {
            method,
            args,
            deadline_unix_ms: deadline_ms,
        };
        let json = serde_json::to_value(&payload)?;
        self.queue
            .insert_lane(REQUEST_LANE, event, &request_id, &json)
            .map_err(|e| RecallError::Other(format!("queue insert: {e}")))?;
        await_response(&self.queue, &request_id, timeout).await
    }
}

fn unix_ms_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

async fn await_response(
    queue: &Arc<Queue>,
    request_id: &str,
    timeout: Duration,
) -> Result<serde_json::Value, RecallError> {
    let start = Instant::now();
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    loop {
        interval.tick().await;
        if let Some(result) = try_take_response(queue, request_id)? {
            return Ok(result);
        }
        if start.elapsed() >= timeout {
            return Err(RecallError::DeadlineExceeded {
                millis: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
            });
        }
    }
}

/// Demux the response lane by `request_id` so concurrent callers do not
/// block on each other's head-of-lane row.
///
/// Uses [`Queue::take_response_for`] which selects + marks-done atomically
/// inside one `BEGIN IMMEDIATE` transaction — no `peek` + `complete` race
/// window.
fn try_take_response(
    queue: &Arc<Queue>,
    request_id: &str,
) -> Result<Option<serde_json::Value>, RecallError> {
    let Some((_id, payload)) = queue
        .take_response_for(RESPONSE_LANE, request_id)
        .map_err(|e| RecallError::Other(format!("queue take_response: {e}")))?
    else {
        return Ok(None);
    };
    let parsed: ResponsePayload = serde_json::from_str(&payload)?;
    if !parsed.ok {
        let msg = parsed.error.map_or_else(|| "unknown".into(), |e| e.msg);
        return Err(RecallError::Other(format!("python sidecar: {msg}")));
    }
    Ok(Some(parsed.result))
}

#[async_trait]
impl RecallBackend for IpcBackend {
    async fn index_page(
        &self,
        page_id: &str,
        content: &str,
        source_path: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), RecallError> {
        self.bm25
            .index_page(page_id, content, source_path, metadata)
            .await?;
        let args = serde_json::json!({
            "page_id": page_id,
            "content": content,
            "source_path": source_path,
            "metadata": metadata,
        });
        if let Err(e) = self
            .round_trip(INDEX_PAGE_EVENT, "index_page", args, DEFAULT_INDEX_TIMEOUT)
            .await
        {
            tracing::warn!(error = %e, "python sidecar index_page skipped");
        }
        Ok(())
    }

    async fn index_session(
        &self,
        _session_id: &str,
        _jsonl_path: &Path,
    ) -> Result<(), RecallError> {
        Err(RecallError::Other(
            "IpcBackend session ingest is owned by the workspace indexer".into(),
        ))
    }

    async fn remove(&self, doc_id: &str) -> Result<(), RecallError> {
        self.bm25.remove(doc_id).await?;
        let args = serde_json::json!({"doc_id": doc_id});
        if let Err(e) = self
            .round_trip(REMOVE_EVENT, "remove", args, DEFAULT_INDEX_TIMEOUT)
            .await
        {
            tracing::warn!(error = %e, "python sidecar remove skipped");
        }
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        k: usize,
        filters: &Filters,
        streams: StreamSet,
    ) -> Result<Vec<Hit>, RecallError> {
        if streams.is_empty() {
            return Err(RecallError::InvalidArgument("empty stream set".into()));
        }
        if !streams.contains(Stream::Vector) && !streams.contains(Stream::Graph) {
            return self.bm25.search(query, k, filters, streams).await;
        }
        let args = serde_json::json!({
            "query": query,
            "k": k,
            "filters": filters,
            "streams": streams.iter().map(Stream::as_str).collect::<Vec<_>>(),
        });
        match self
            .round_trip(SEARCH_EVENT, "search", args, DEFAULT_SEARCH_TIMEOUT)
            .await
        {
            Ok(v) => parse_hits(&v),
            Err(e) => degrade_to_bm25(&self.bm25, query, k, filters, &e).await,
        }
    }

    async fn health_check(&self) -> Result<serde_json::Value, RecallError> {
        match self
            .round_trip(HEALTH_EVENT, "health_check", serde_json::json!({}), DEFAULT_HEALTH_TIMEOUT)
            .await
        {
            Ok(v) => Ok(v),
            Err(_) => Ok(serde_json::json!({"backend": "ipc", "python": "down"})),
        }
    }
}

async fn degrade_to_bm25(
    bm25: &Arc<Bm25Backend>,
    query: &str,
    k: usize,
    filters: &Filters,
    err: &RecallError,
) -> Result<Vec<Hit>, RecallError> {
    tracing::warn!(error = %err, "python sidecar unreachable; degrading to BM25-only");
    bm25.search(query, k, filters, StreamSet::bm25_only()).await
}

fn parse_hits(value: &serde_json::Value) -> Result<Vec<Hit>, RecallError> {
    let arr = value
        .get("hits")
        .and_then(|v| v.as_array())
        .ok_or_else(|| RecallError::Other("response missing `hits`".into()))?;
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let hit: Hit = serde_json::from_value(v.clone())?;
        out.push(hit);
    }
    Ok(out)
}
