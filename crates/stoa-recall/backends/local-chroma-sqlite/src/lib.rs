//! Stoa default recall backend: `ChromaDB` + `SQLite` FTS5 + KG.
//!
//! ARCHITECTURE §6.1: hybrid recall over three streams. This crate owns
//! the Rust side: the FTS5 + KG schema for `.stoa/recall.db`, a BM25-only
//! search path that works without the Python sidecar, and an IPC backend
//! that speaks to the Python `stoa-recall` worker over the queue lanes
//! `recall.request` / `recall.response`.
//!
//! Two `RecallBackend` impls live here:
//!
//! - [`Bm25Backend`] — `rusqlite` only. Always available. Powers
//!   `stoa init --no-embeddings` and the `streams=bm25` fast path.
//! - [`IpcBackend`] — proxies to the Python sidecar via the queue.
//!   Falls back to [`Bm25Backend`] for BM25-only requests so single-stream
//!   queries succeed even when the sidecar is down.

mod bm25;
mod ipc;
mod sanitize;
mod schema;

pub use bm25::Bm25Backend;
pub use ipc::{IpcBackend, REQUEST_LANE, RESPONSE_LANE, SEARCH_LANE};
pub use schema::{RECALL_DB_FILE, ensure_schema};
