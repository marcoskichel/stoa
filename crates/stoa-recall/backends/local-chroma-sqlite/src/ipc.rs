//! IPC backend — proxies recall calls to the Python sidecar via the queue.
//!
//! Lanes:
//!
//! - `recall.request` — Rust → Python for write-side ops
//!   (`recall.index_page`, `recall.remove`, `recall.health_check`).
//!   Also drained by the Rust daemon for BM25 reindex on `index_page`
//!   / `remove_page` so single-stream queries succeed without the
//!   sidecar.
//! - `recall.search` — Rust → Python for read-side ops only. The Rust
//!   daemon never claims this lane; only the Python sidecar drains it.
//!   Splitting reads from writes prevents a daemon-claim → release
//!   livelock when the sidecar is offline.
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

/// Write-side request lane (`index_page`, `remove`, `health_check`).
///
/// Drained by both the Rust daemon (BM25 reindex) and the Python
/// sidecar (vector / KG ack); rows on this lane are routable to either
/// pool by `method`.
pub const REQUEST_LANE: &str = "recall.request";

/// Read-side request lane (`search` only).
///
/// Drained exclusively by the Python sidecar. The Rust daemon never
/// claims this lane — if the sidecar is offline the row stays pending
/// until the request timeout expires, then the caller degrades to
/// BM25-only via the in-process fallback path.
pub const SEARCH_LANE: &str = "recall.search";

/// Lane the Python sidecar writes onto with the result.
pub const RESPONSE_LANE: &str = "recall.response";

const SEARCH_EVENT: &str = "recall.search";
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

    /// Re-enqueue a `recall.request` row so the daemon's drainer (or a
    /// future sidecar liveness recovery) re-attempts the work. Without
    /// this the BM25 stream + the vector / KG stream silently diverge
    /// — BM25 forgets the doc, the sidecar's view never updates.
    fn reenqueue_remove(&self, doc_id: &str, args: &serde_json::Value) -> Result<(), RecallError> {
        let payload = serde_json::json!({
            "method": "remove",
            "args": args,
        });
        let session_id = format!("retry:remove:{doc_id}:{}", Uuid::new_v4());
        self.queue
            .insert_lane(REQUEST_LANE, "recall.remove.retry", &session_id, &payload)
            .map_err(|e| RecallError::Other(format!("re-enqueue remove: {e}")))
    }

    /// Send the request described by `call` on its lane, await the
    /// response, return the raw `result` JSON.
    ///
    /// `call.lane` selects which sidecar pool will service the row.
    /// Reads (`search`) MUST use [`SEARCH_LANE`] so they bypass the
    /// Rust daemon; writes (`index_page`, `remove`, `health_check`)
    /// use [`REQUEST_LANE`] so the Rust daemon can ack the BM25-only
    /// path without the Python sidecar.
    async fn round_trip(&self, call: RoundTrip) -> Result<serde_json::Value, RecallError> {
        let request_id = Uuid::new_v4().to_string();
        let deadline_ms = unix_ms_now().saturating_add(call.timeout.as_millis());
        let payload = RequestPayload {
            method: call.method,
            args: call.args,
            deadline_unix_ms: deadline_ms,
        };
        let json = serde_json::to_value(&payload)?;
        self.queue
            .insert_lane(call.lane, call.event, &request_id, &json)
            .map_err(|e| RecallError::Other(format!("queue insert: {e}")))?;
        await_response(&self.queue, &request_id, call.timeout).await
    }
}

/// Parameters for one `round_trip` IPC call.
struct RoundTrip {
    lane: &'static str,
    event: &'static str,
    method: &'static str,
    args: serde_json::Value,
    timeout: Duration,
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
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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
    /// `IpcBackend` defers vector / KG ingest to the daemon's
    /// `recall.request` drainer (see `crates/stoa-cli/src/daemon`). The
    /// trait method here only updates the BM25 stream so direct
    /// callers (the workspace indexer, tests) cannot accidentally
    /// dual-write through both the trait + the queue.
    async fn index_page(
        &self,
        page_id: &str,
        content: &str,
        source_path: &str,
        metadata: &serde_json::Value,
    ) -> Result<(), RecallError> {
        self.bm25
            .index_page(page_id, content, source_path, metadata)
            .await
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
        let call = RoundTrip {
            lane: REQUEST_LANE,
            event: REMOVE_EVENT,
            method: "remove",
            args: args.clone(),
            timeout: DEFAULT_INDEX_TIMEOUT,
        };
        if let Err(e) = self.round_trip(call).await {
            tracing::warn!(error = %e, "python sidecar remove failed; re-enqueueing");
            self.reenqueue_remove(doc_id, &args)?;
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
            "streams": streams,
        });
        let call = RoundTrip {
            lane: SEARCH_LANE,
            event: SEARCH_EVENT,
            method: "search",
            args,
            timeout: DEFAULT_SEARCH_TIMEOUT,
        };
        match self.round_trip(call).await {
            Ok(v) => parse_hits(&v),
            Err(e) => degrade_to_bm25(&self.bm25, query, k, filters, &e).await,
        }
    }

    async fn health_check(&self) -> Result<serde_json::Value, RecallError> {
        let call = RoundTrip {
            lane: REQUEST_LANE,
            event: HEALTH_EVENT,
            method: "health_check",
            args: serde_json::json!({}),
            timeout: DEFAULT_HEALTH_TIMEOUT,
        };
        match self.round_trip(call).await {
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
